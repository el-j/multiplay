//! Embeds the app icon into the Windows executable. The `.res` is written in pure Rust from the
//! committed PNGs and passed to the MSVC linker, which accepts compiled resources directly, so no
//! resource compiler (rc.exe, windres) is needed. Also exports the release version and commit.

#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "src/app_icon/formats.rs"]
mod formats;

#[path = "../../build-support/build_info.rs"]
mod build_info;

use std::path::PathBuf;

fn main() {
    build_info::export();
    println!("cargo:rerun-if-changed=../../assets/icon");
    println!("cargo:rerun-if-changed=src/app_icon/formats.rs");
    let target = |key: &str| std::env::var(key).unwrap_or_default();
    if target("CARGO_CFG_TARGET_OS") != "windows" || target("CARGO_CFG_TARGET_ENV") != "msvc" {
        return;
    }
    let assets = PathBuf::from(target("CARGO_MANIFEST_DIR")).join("../../assets/icon");
    let pngs: Vec<(u32, Vec<u8>)> = formats::WINDOWS_SIZES
        .iter()
        .map(|&s| {
            let path = assets.join(format!("multiplay-{s}.png"));
            let png =
                std::fs::read(&path).unwrap_or_else(|e| panic!("app icon {}: {e}", path.display()));
            (s, png)
        })
        .collect();
    let refs: Vec<_> = pngs.iter().map(|(s, p)| (*s, p.as_slice())).collect();
    let res = PathBuf::from(target("OUT_DIR")).join("multiplay.res");
    std::fs::write(&res, formats::res(&refs)).expect("write multiplay.res");
    println!("cargo:rustc-link-arg-bin=multiplay-gui={}", res.display());
}
