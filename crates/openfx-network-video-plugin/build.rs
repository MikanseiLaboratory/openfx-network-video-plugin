use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=NDI_SDK_DIR");

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("windows") {
        println!("cargo:rustc-link-arg=/DELAYLOAD:Processing.NDI.Lib.x64.dll");
        println!("cargo:rustc-link-lib=delayimp");
    }

    copy_ndi_runtime_for_tests();
}

fn copy_ndi_runtime_for_tests() {
    let Some(dll) = ndi_runtime_dll() else {
        return;
    };
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());
    let Some(profile_dir) = out_dir.ancestors().nth(3) else {
        return;
    };
    let _ = fs::copy(&dll, profile_dir.join("Processing.NDI.Lib.x64.dll"));
    let deps = profile_dir.join("deps");
    if deps.is_dir() {
        let _ = fs::copy(&dll, deps.join("Processing.NDI.Lib.x64.dll"));
    }
}

fn ndi_runtime_dll() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(sdk) = env::var("NDI_SDK_DIR") {
        candidates.push(PathBuf::from(sdk).join("Bin/x64/Processing.NDI.Lib.x64.dll"));
    }
    candidates.push(PathBuf::from(
        r"C:\Program Files\NDI\NDI 6 SDK\Bin\x64\Processing.NDI.Lib.x64.dll",
    ));
    candidates.push(PathBuf::from(
        r"C:\Program Files\NDI\NDI 6 Runtime\v6\Processing.NDI.Lib.x64.dll",
    ));
    candidates
        .into_iter()
        .find(|path| Path::new(path).is_file())
}
