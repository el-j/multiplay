//! Pure-Rust Runtime Composition Root binary entrypoint for Multiplay.

#![forbid(unsafe_code)]

use amp_app::{runner, MultiplayApp};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (app, cmd_rx) = MultiplayApp::new().map_err(|e| format!("Initialization error: {e}"))?;
    runner::run_app(app, cmd_rx)
}
