//! The Multiplay app icon. The PNGs in `assets/icon` are rendered by
//! `cargo run -p amp-adapter-gui --example make_icons`; the window icon and the OS icon files
//! (`.icns`, `.ico`, and the Windows `.res` made by `build.rs`) are built from them.

#![forbid(unsafe_code)]

pub mod formats;

pub use formats::WINDOWS_SIZES;

use eframe::egui::IconData;
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Every size kept in `assets/icon` as `multiplay-<size>.png`.
pub const SIZES: [u32; 9] = [16, 24, 32, 48, 64, 128, 256, 512, 1024];
/// macOS `.icns` chunk types and the PNG size each one holds (@2x types hold double size).
pub const ICNS_CHUNKS: [([u8; 4], u32); 9] = [
    (*b"icp4", 16),
    (*b"icp5", 32),
    (*b"ic11", 32),
    (*b"ic12", 64),
    (*b"ic07", 128),
    (*b"ic08", 256),
    (*b"ic13", 256),
    (*b"ic09", 512),
    (*b"ic14", 512),
];

const WINDOW_PNG: &[u8] = include_bytes!("../../../../assets/icon/multiplay-256.png");

/// Icon for the window title bar, taskbar and task switcher (Windows and Linux; macOS uses the
/// bundle icon).
pub fn window_icon() -> IconData {
    match image::load_from_memory_with_format(WINDOW_PNG, image::ImageFormat::Png) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            IconData {
                width: rgba.width(),
                height: rgba.height(),
                rgba: rgba.into_raw(),
            }
        }
        Err(_) => IconData::default(),
    }
}

/// The window icon, decoded once and shared by every viewport (they are rebuilt each frame).
pub fn shared_window_icon() -> Arc<IconData> {
    static ICON: OnceLock<Arc<IconData>> = OnceLock::new();
    Arc::clone(ICON.get_or_init(|| Arc::new(window_icon())))
}

fn read_sizes(dir: &Path, sizes: &[u32]) -> std::io::Result<Vec<(u32, Vec<u8>)>> {
    sizes
        .iter()
        .map(|&s| Ok((s, std::fs::read(dir.join(format!("multiplay-{s}.png")))?)))
        .collect()
}

/// `.ico` from the PNGs in `dir`.
pub fn build_ico(dir: &Path) -> std::io::Result<Vec<u8>> {
    let pngs = read_sizes(dir, &WINDOWS_SIZES)?;
    let refs: Vec<_> = pngs.iter().map(|(s, p)| (*s, p.as_slice())).collect();
    Ok(formats::ico(&refs))
}

/// `.icns` from the PNGs in `dir` (the 1024 px image goes in as `ic10`).
pub fn build_icns(dir: &Path) -> std::io::Result<Vec<u8>> {
    let mut chunks = Vec::new();
    for (kind, size) in ICNS_CHUNKS.iter().copied().chain([(*b"ic10", 1024)]) {
        chunks.push((
            kind,
            std::fs::read(dir.join(format!("multiplay-{size}.png")))?,
        ));
    }
    let refs: Vec<_> = chunks.iter().map(|(k, p)| (*k, p.as_slice())).collect();
    Ok(formats::icns(&refs))
}
