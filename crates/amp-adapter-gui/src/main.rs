//! Native Desktop GUI Entrypoint for Multiplay.
//! Pure-Rust, GPU-accelerated (Metal/Vulkan/DirectX) multi-viewport native GUI powered by egui.
//! Zero NodeJS, zero TypeScript, zero Electron.

#![forbid(unsafe_code)]
// Release builds on Windows are a GUI app: no console window opens behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use amp_adapter_gui::chrome::classic_viewport;
use amp_adapter_gui::MultiplayGuiApp;
use amp_core_engine::diagnostics::{self, AppInfo};
use eframe::egui::{Pos2, Vec2};

fn main() -> eframe::Result<()> {
    if std::env::args()
        .skip(1)
        .any(|a| a == "--version" || a == "-V")
    {
        println!("{}", amp_adapter_gui::about::version_line());
        return Ok(());
    }
    let logs = amp_adapter_gui::app_paths::AppPaths::for_user().log_dir();
    // Kept alive until the window closes so buffered log lines are flushed on exit.
    let _logging = diagnostics::init_logging(&logs, "info");
    let app_info = AppInfo {
        name: "multiplay",
        version_line: amp_adapter_gui::about::version_line(),
    };
    tracing::info!(
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        "{} starting",
        app_info.version_line
    );
    diagnostics::install_crash_hook(app_info, logs, None);
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
        Box::new(|cc| {
            let mut app = MultiplayGuiApp::new().expect("Failed to initialize MultiplayGuiApp");
            attach_media_controls(&mut app, cc);
            Ok(Box::new(app))
        }),
    )
}

/// Media keys and Now Playing. Without a media session (no D-Bus session on Linux, say) the app
/// works as before, minus the keys.
fn attach_media_controls(app: &mut MultiplayGuiApp, cc: &eframe::CreationContext<'_>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let hwnd = cc.window_handle().ok().and_then(|h| match h.as_raw() {
        RawWindowHandle::Win32(w) => Some(std::ptr::without_provenance_mut(w.hwnd.get() as usize)),
        _ => None,
    });
    let ctx = cc.egui_ctx.clone();
    match amp_adapter_media_controls::OsMediaControls::new(
        hwnd,
        Box::new(move || ctx.request_repaint()),
    ) {
        Ok(controls) => app.attach_media_controls(Box::new(controls)),
        Err(e) => tracing::info!("media keys unavailable: {e}"),
    }
}
