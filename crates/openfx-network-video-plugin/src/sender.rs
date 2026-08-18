use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use grafton_ndi::{BorrowedVideoFrame, NDI, PixelFormat, Sender, SenderOptions};
use windows_sys::Win32::Foundation::{GetLastError, HMODULE, MAX_PATH};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW, SetDllDirectoryW,
};

use crate::config::PluginConfig;
use crate::media::{PixelFormatKind, pixel_format_kind};
use openfx_pixels::{ConvertedVideo, PixelPool, packed_frame_hash};

const NDI_DLL: &str = "Processing.NDI.Lib.x64.dll";

#[derive(Debug, Clone)]
pub struct VideoJob {
    pub width: u32,
    pub height: u32,
    pub stride: i32,
    pub rgba: Vec<u8>,
    pub pixel_format: PixelFormatKind,
    pub timecode: i64,
    pub fps_n: i32,
    pub fps_d: i32,
    pub ofx_time: f64,
}

impl From<ConvertedVideo> for VideoJob {
    fn from(value: ConvertedVideo) -> Self {
        Self {
            width: value.width,
            height: value.height,
            stride: value.stride,
            pixel_format: pixel_format_kind(value.has_alpha),
            rgba: value.data,
            timecode: 0,
            fps_n: 60,
            fps_d: 1,
            ofx_time: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct LatestSlot<T> {
    slot: Mutex<Option<T>>,
    drops: AtomicU64,
}

impl<T> LatestSlot<T> {
    pub fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            drops: AtomicU64::new(0),
        }
    }

    pub fn push(&self, item: T) {
        drop(self.push_replacing(item));
    }

    pub fn push_replacing(&self, item: T) -> Option<T> {
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        let old = slot.replace(item);
        if old.is_some() {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
        old
    }

    pub fn take(&self) -> Option<T> {
        self.slot.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub fn clear(&self) {
        let _ = self.take();
    }

    pub fn drops(&self) -> u64 {
        self.drops.load(Ordering::Relaxed)
    }
}

impl<T> Default for LatestSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SendSession {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    video_slot: Arc<LatestSlot<VideoJob>>,
    pool: Arc<PixelPool>,
}

impl SendSession {
    pub fn start(config: PluginConfig) -> Result<Self, String> {
        Self::start_with_pool(config, Arc::new(PixelPool::new()))
    }

    pub fn start_with_pool(config: PluginConfig, pool: Arc<PixelPool>) -> Result<Self, String> {
        prepare_ndi_runtime()?;
        let ndi = NDI::new().map_err(|e| format!("NDI runtime init failed: {e}"))?;
        let mut builder = SenderOptions::builder(config.source_name.clone()).clock_video(false);
        if let Some(groups) = config.groups_opt() {
            builder = builder.groups(groups);
        }
        let options = builder.build();
        let sender =
            Sender::new(&ndi, &options).map_err(|e| format!("NDI sender create failed: {e}"))?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let video_slot = Arc::new(LatestSlot::new());
        let video_slot_thread = Arc::clone(&video_slot);
        let pool_thread = Arc::clone(&pool);

        let join = thread::Builder::new()
            .name("openfx-ndi-sender".into())
            .spawn(move || {
                sender_loop(ndi, sender, video_slot_thread, pool_thread, stop_thread);
            })
            .map_err(|e| format!("failed to spawn NDI sender thread: {e}"))?;

        Ok(Self {
            stop,
            join: Some(join),
            video_slot,
            pool,
        })
    }

    pub fn push_video(&self, job: VideoJob) {
        if let Some(old) = self.video_slot.push_replacing(job) {
            self.pool.release(old.rgba);
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        if let Some(job) = self.video_slot.take() {
            self.pool.release(job.rgba);
        }
    }
}

impl Drop for SendSession {
    fn drop(&mut self) {
        self.stop();
    }
}

fn sender_loop(
    _ndi: NDI,
    mut sender: Sender,
    video_slot: Arc<LatestSlot<VideoJob>>,
    pool: Arc<PixelPool>,
    stop: Arc<AtomicBool>,
) {
    let mut dedup = SendDedupState::default();
    let mut pending: Option<VideoJob> = None;
    while !stop.load(Ordering::Acquire) {
        let job = pending.take().or_else(|| video_slot.take());
        let Some(job) = job else {
            thread::sleep(Duration::from_millis(1));
            continue;
        };
        let sent = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            send_job(&mut sender, &video_slot, &pool, &stop, job, &mut dedup)
        }));
        match sent {
            Ok(next) => pending = next,
            Err(_) => {
                eprintln!("NDI sender loop panicked; keeping sender thread alive");
                thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

struct SendDedupState {
    time: f64,
    wh: (u32, u32),
    hash: u64,
}

impl Default for SendDedupState {
    fn default() -> Self {
        Self {
            time: f64::NAN,
            wh: (0, 0),
            hash: 0,
        }
    }
}

fn send_job(
    sender: &mut Sender,
    video_slot: &LatestSlot<VideoJob>,
    pool: &PixelPool,
    stop: &AtomicBool,
    job: VideoJob,
    dedup: &mut SendDedupState,
) -> Option<VideoJob> {
    if job.ofx_time == dedup.time && dedup.wh == (job.width, job.height) {
        let hash = packed_frame_hash(job.width, job.height, &job.rgba);
        if hash == dedup.hash {
            pool.release(job.rgba);
            return None;
        }
        dedup.hash = hash;
    } else {
        dedup.time = job.ofx_time;
        dedup.wh = (job.width, job.height);
        dedup.hash = 0;
    }

    let pixel_format = match job.pixel_format {
        PixelFormatKind::Rgba => PixelFormat::RGBA,
        PixelFormatKind::Rgbx => PixelFormat::RGBX,
    };
    let borrowed = match BorrowedVideoFrame::try_from_uncompressed(
        &job.rgba,
        job.width as i32,
        job.height as i32,
        pixel_format,
        job.fps_n,
        job.fps_d,
    ) {
        Ok(frame) => frame,
        Err(e) => {
            eprintln!("NDI video frame failed: {e}");
            pool.release(job.rgba);
            return None;
        }
    };
    let token = sender.send_video_async(&borrowed);
    let next = loop {
        if stop.load(Ordering::Acquire) {
            break None;
        }
        if let Some(next) = video_slot.take() {
            break Some(next);
        }
        thread::sleep(Duration::from_millis(1));
    };
    drop(token);
    pool.release(job.rgba);
    next
}

pub fn prepare_ndi_runtime() -> Result<PathBuf, String> {
    let dir = find_ndi_runtime_dir()?;
    set_dll_directory(&dir)?;
    let dll = dir.join(NDI_DLL);
    if !dll.is_file() {
        return Err(format!(
            "NDI runtime DLL not found at {}. Place {NDI_DLL} next to the plugin.",
            dll.display()
        ));
    }
    Ok(dll)
}

fn find_ndi_runtime_dir() -> Result<PathBuf, String> {
    let mut dirs = Vec::new();
    if let Ok(plugin_dir) = current_module_dir() {
        dirs.push(plugin_dir);
    }
    if let Ok(sdk) = std::env::var("NDI_SDK_DIR") {
        dirs.push(PathBuf::from(sdk).join("Bin/x64"));
    }
    if let Ok(runtime) = std::env::var("NDI_RUNTIME_DIR_V6") {
        dirs.push(PathBuf::from(runtime));
    }
    dirs.push(PathBuf::from(r"C:\Program Files\NDI\NDI 6 SDK\Bin\x64"));
    dirs.push(PathBuf::from(r"C:\Program Files\NDI\NDI 6 Runtime\v6"));

    dirs.into_iter()
        .find(|dir| dir.join(NDI_DLL).is_file())
        .ok_or_else(|| {
            format!("NDI runtime ({NDI_DLL}) was not found next to the plugin or in NDI_SDK_DIR")
        })
}

fn current_module_dir() -> Result<PathBuf, String> {
    unsafe {
        let mut module: HMODULE = std::ptr::null_mut();
        let ok = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            current_module_dir as *const () as *const u16,
            &mut module,
        );
        if ok == 0 || module.is_null() {
            return Err(format!("GetModuleHandleExW failed ({})", GetLastError()));
        }
        let mut buf = [0u16; MAX_PATH as usize + 1];
        let len = GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32);
        if len == 0 {
            return Err(format!("GetModuleFileNameW failed ({})", GetLastError()));
        }
        let path = PathBuf::from(OsString::from_wide(&buf[..len as usize]));
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "plugin path has no parent directory".into())
    }
}

fn set_dll_directory(dir: &Path) -> Result<(), String> {
    let mut wide: Vec<u16> = dir.as_os_str().encode_wide().collect();
    wide.push(0);
    let ok = unsafe { SetDllDirectoryW(wide.as_ptr()) };
    if ok == 0 {
        Err(format!(
            "SetDllDirectoryW({}) failed ({})",
            dir.display(),
            unsafe { GetLastError() }
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_slot_drops_old() {
        let slot = LatestSlot::new();
        slot.push(1);
        slot.push(2);
        assert_eq!(slot.take(), Some(2));
        assert_eq!(slot.drops(), 1);
        assert_eq!(slot.take(), None);
    }

    #[test]
    fn latest_wins_under_contention() {
        let slot = Arc::new(LatestSlot::new());
        let mut handles = Vec::new();
        for i in 0..8 {
            let slot = Arc::clone(&slot);
            handles.push(thread::spawn(move || {
                for j in 0..50 {
                    slot.push(i * 100 + j);
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert!(slot.take().is_some());
    }

    #[test]
    fn stop_is_idempotent() {
        let mut session = SendSession {
            stop: Arc::new(AtomicBool::new(false)),
            join: None,
            video_slot: Arc::new(LatestSlot::new()),
            pool: Arc::new(PixelPool::new()),
        };
        session.stop();
        session.stop();
        assert!(session.stop.load(Ordering::Acquire));
    }

    #[test]
    fn send_session_start_and_stop() {
        let Ok(mut session) = SendSession::start(PluginConfig {
            enabled: true,
            source_name: "openfx-ndi-stop-test".into(),
            groups: String::new(),
        }) else {
            return;
        };
        session.push_video(VideoJob {
            width: 16,
            height: 16,
            stride: 64,
            rgba: vec![0u8; 16 * 16 * 4],
            pixel_format: PixelFormatKind::Rgbx,
            timecode: 1,
            fps_n: 60,
            fps_d: 1,
            ofx_time: 0.0,
        });
        session.stop();
        session.stop();
    }
}
