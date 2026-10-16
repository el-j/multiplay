//! Native Desktop GUI Entrypoint for Multiplay.
//! Pure-Rust, GPU-accelerated (Metal/Vulkan/DirectX) multi-viewport native GUI powered by egui.
//! Zero NodeJS, zero TypeScript, zero Electron.

#![forbid(unsafe_code)]

use amp_adapter_gui::MultiplayGuiApp;
use eframe::egui::{Pos2, Vec2, ViewportBuilder};

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("WINAMP :: MAIN PLAYER")
            .with_inner_size(Vec2::new(480.0, 190.0))
            .with_min_inner_size(Vec2::new(380.0, 180.0))
            .with_position(Pos2::new(120.0, 100.0))
            .with_resizable(true)
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "WINAMP :: MAIN PLAYER",
        native_options,
        Box::new(|_cc| {
            let app = MultiplayGuiApp::new().expect("Failed to initialize MultiplayGuiApp");
            Ok(Box::new(app))
        }),
    )
}
