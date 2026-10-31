//! Pure-Rust Runtime Composition Root binary entrypoint for Multiplay.

#![forbid(unsafe_code)]

use amp_app::{runner, MultiplayApp};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args()
        .skip(1)
        .any(|a| a == "--version" || a == "-V")
    {
        let line = amp_domain::about::version_line(
            "multiplay-tui",
            env!("MULTIPLAY_VERSION"),
            env!("MULTIPLAY_COMMIT"),
        );
        println!("{line}");
        return Ok(());
    }
    let (app, cmd_rx) = MultiplayApp::new().map_err(|e| format!("Initialization error: {e}"))?;
    runner::run_app(app, cmd_rx)
}
