//! Native Desktop GUI Entrypoint for Multiplay.
//! Pure-Rust, GPU-accelerated (Metal/Vulkan/DirectX) immediate-mode GUI powered by egui.
//! Zero NodeJS, zero TypeScript, zero Electron.

#![forbid(unsafe_code)]

mod app;
mod audio_driver;
mod dj_panel;
mod equalizer_widget;
mod settings_panel;
mod visualizer_widget;

use app::MultiplayGuiApp;
use eframe::egui::Vec2;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(560.0, 580.0))
            .with_min_inner_size(Vec2::new(480.0, 420.0))
            .with_title("Multiplay Pure-Rust Audiophile Player")
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Multiplay Pure-Rust Audiophile Player",
        native_options,
        Box::new(|_cc| {
            let app = MultiplayGuiApp::new().expect("Failed to initialize MultiplayGuiApp");
            Ok(Box::new(app))
        }),
    )
}
