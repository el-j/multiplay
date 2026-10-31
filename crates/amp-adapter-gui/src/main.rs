//! Native Desktop GUI Entrypoint for Multiplay.
//! Pure-Rust, GPU-accelerated (Metal/Vulkan/DirectX) multi-viewport native GUI powered by egui.
//! Zero NodeJS, zero TypeScript, zero Electron.

#![forbid(unsafe_code)]
// Release builds on Windows are a GUI app: no console window opens behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use amp_adapter_gui::chrome::classic_viewport;
use amp_adapter_gui::MultiplayGuiApp;
use eframe::egui::{Pos2, Vec2};

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        // The app id ties the window to `multiplay.desktop` (and its icon) on Wayland and X11.
        viewport: classic_viewport("multiplay", false)
            .with_app_id("multiplay")
            .with_inner_size(Vec2::new(480.0, 210.0))
            .with_position(Pos2::new(120.0, 100.0))
            .with_active(true),
        multisampling: 0,
        ..Default::default()
    };

    eframe::run_native(
        "multiplay",
        native_options,
        Box::new(|_cc| {
            let app = MultiplayGuiApp::new().expect("Failed to initialize MultiplayGuiApp");
            Ok(Box::new(app))
        }),
    )
}
