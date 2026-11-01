//! Pure-Rust Runtime Composition Root binary entrypoint for Multiplay.

#![forbid(unsafe_code)]

use amp_app::{runner, terminal, MultiplayApp};
use amp_core_engine::diagnostics::{self, AppInfo};
use std::path::PathBuf;

/// `~/.multiplay/logs`, shared with the desktop app.
fn log_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join(".multiplay/logs")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args()
        .skip(1)
        .any(|a| a == "--version" || a == "-V")
    {
        println!("{}", version_line());
        return Ok(());
    }
    let logs = log_dir();
    let _logging = diagnostics::init_logging(&logs, "info");
    let app_info = AppInfo {
        name: "multiplay-tui",
        version_line: version_line(),
    };
    tracing::info!(
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        "{} starting",
        app_info.version_line
    );
    diagnostics::install_crash_hook(app_info, logs, Some(Box::new(terminal::restore_terminal)));
    let (app, cmd_rx) = MultiplayApp::new().map_err(|e| format!("Initialization error: {e}"))?;
    #[cfg(all(unix, not(target_os = "macos")))]
    let app = with_media_keys(app);
    runner::run_app(app, cmd_rx)
}

fn version_line() -> String {
    amp_domain::about::version_line(
        "multiplay-tui",
        env!("MULTIPLAY_VERSION"),
        env!("MULTIPLAY_COMMIT"),
    )
}

/// MPRIS media keys on Linux. macOS needs an AppKit run loop and Windows a window, which a
/// terminal app has neither of, so media keys there belong to the desktop app.
#[cfg(all(unix, not(target_os = "macos")))]
fn with_media_keys(mut app: MultiplayApp) -> MultiplayApp {
    match amp_adapter_media_controls::OsMediaControls::new(None, Box::new(|| {})) {
        Ok(controls) => app.media_controls = Some(Box::new(controls)),
        Err(e) => tracing::info!("media keys unavailable: {e}"),
    }
    app
}
