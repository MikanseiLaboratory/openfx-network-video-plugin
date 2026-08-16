//! NDI® sender/receiver loopback for synthetic RGBA video.

use std::thread;
use std::time::{Duration, Instant};

use grafton_ndi::{
    Finder, FinderOptions, NDI, PixelFormat, Receiver, ReceiverBandwidth, ReceiverColorFormat,
    ReceiverOptions, Sender, SenderOptions, VideoFrame,
};
use openfx::image::{PixelComponents, PixelDepth, RectI};
use openfx_network_video::{ConvertedVideo, convert_window_to_rgba, prepare_ndi_runtime};

fn require_ndi() -> Option<NDI> {
    if let Err(e) = prepare_ndi_runtime() {
        eprintln!("skipping NDI loopback: {e}");
        return None;
    }
    match NDI::new() {
        Ok(ndi) => Some(ndi),
        Err(e) => {
            eprintln!("skipping NDI loopback: runtime init failed: {e}");
            None
        }
    }
}

fn find_source(ndi: &NDI, needle: &str) -> grafton_ndi::Source {
    let finder = Finder::new(
        ndi,
        &FinderOptions::builder().show_local_sources(true).build(),
    )
    .expect("finder");
    for _ in 0..40 {
        let _ = finder.wait_for_sources(Duration::from_millis(250));
        if let Ok(sources) = finder.current_sources()
            && let Some(source) = sources.into_iter().find(|s| s.name.contains(needle))
        {
            return source;
        }
    }
    panic!("did not discover NDI source containing {needle}");
}

fn wait_connected(sender: &Sender, connected: bool) {
    for _ in 0..80 {
        let count = sender.connection_count(Duration::ZERO).unwrap_or(0);
        if connected && count > 0 || !connected && count == 0 {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn solid_rgba(size: u32, r: u8, g: u8, b: u8, a: u8) -> ConvertedVideo {
    let window = RectI {
        x1: 0,
        y1: 0,
        x2: size as i32,
        y2: size as i32,
    };
    let mut src = vec![0u8; (size * size * 4) as usize];
    for px in src.chunks_exact_mut(4) {
        px.copy_from_slice(&[r, g, b, a]);
    }
    unsafe {
        convert_window_to_rgba(
            window,
            window,
            (size * 4) as i32,
            src.as_ptr(),
            PixelDepth::Byte,
            PixelComponents::Rgba,
        )
    }
    .expect("convert")
}

fn video_frame(converted: ConvertedVideo, timecode: i64) -> VideoFrame {
    let format = if converted.has_alpha {
        PixelFormat::RGBA
    } else {
        PixelFormat::RGBX
    };
    let mut frame = VideoFrame::builder()
        .resolution(converted.width as i32, converted.height as i32)
        .pixel_format(format)
        .frame_rate(60, 1)
        .timecode(timecode)
        .build()
        .expect("video frame");
    frame.replace_data(converted.data).expect("replace");
    frame
}

fn recv_video_with_timecode(
    sender: &Sender,
    receiver: &Receiver,
    frame: &VideoFrame,
    timecode: i64,
) -> VideoFrame {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        sender.send_video(frame);
        if let Ok(Some(got)) = receiver.video().try_capture(Duration::from_millis(50))
            && got.timecode() == timecode
        {
            return got;
        }
    }
    panic!("did not receive video with timecode {timecode}");
}

#[test]
fn rgba_loopback_timecode_and_reconnect() {
    let Some(ndi) = require_ndi() else {
        return;
    };
    let name = format!("openfx-ndi-loopback-{}", std::process::id());
    let sender = Sender::new(
        &ndi,
        &SenderOptions::builder(&name).clock_video(false).build(),
    )
    .expect("sender");

    let source = find_source(&ndi, &name);
    let receiver = Receiver::new(
        &ndi,
        &ReceiverOptions::builder(source)
            .color(ReceiverColorFormat::RGBX_RGBA)
            .bandwidth(ReceiverBandwidth::Highest)
            .name("openfx-ndi-loopback-rx")
            .build(),
    )
    .expect("receiver");
    wait_connected(&sender, true);

    let converted = solid_rgba(32, 10, 20, 30, 255);
    assert_eq!(converted.width, 32);
    assert_eq!(converted.height, 32);
    let video = video_frame(converted, 1_000_000);
    let got = recv_video_with_timecode(&sender, &receiver, &video, 1_000_000);
    assert_eq!(got.width(), 32);
    assert_eq!(got.height(), 32);
    assert_eq!(got.timecode(), 1_000_000);

    drop(receiver);
    wait_connected(&sender, false);

    let source = find_source(&ndi, &name);
    let receiver2 = Receiver::new(
        &ndi,
        &ReceiverOptions::builder(source)
            .color(ReceiverColorFormat::RGBX_RGBA)
            .bandwidth(ReceiverBandwidth::Highest)
            .name("openfx-ndi-loopback-rx2")
            .build(),
    )
    .expect("reconnect");
    wait_connected(&sender, true);
    let converted = solid_rgba(32, 1, 2, 3, 255);
    let video = recv_video_with_timecode(
        &sender,
        &receiver2,
        &video_frame(converted, 2_000_000),
        2_000_000,
    );
    assert_eq!(video.timecode(), 2_000_000);
}
