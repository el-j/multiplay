//! Main terminal execution loop orchestrating rendering, audio sync, and keyboard events.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayApp;
use crate::terminal::TerminalGuard;
use amp_core_engine::PlayerCommand;
use crossterm::event::{self, Event};
use std::time::Duration;
use tokio::sync::mpsc;

/// Runs the interactive terminal UI loop until user requests exit.
pub fn run_app(
    mut app: MultiplayApp,
    mut cmd_rx: mpsc::UnboundedReceiver<PlayerCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut guard = TerminalGuard::init().map_err(|e| format!("Terminal init failed: {e}"))?;

    loop {
        // Drain incoming background player commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                PlayerCommand::SetVolume(v) => {
                    app.volume = (v * 100.0) as u8;
                }
                PlayerCommand::Stop => {
                    app.playback_state = amp_domain::PlaybackState::Stopped;
                    app.elapsed = Duration::ZERO;
                }
                _ => {}
            }
        }

        // Synchronize real-time audio playback driver position and spectrum visualizer
        app.sync_audio_state();

        // Render frame to alternate terminal buffer
        guard
            .terminal_mut()
            .draw(|f| app.render_frame(f))
            .map_err(|e| format!("Render error: {e}"))?;

        // Poll keyboard input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                if app.handle_key(key_event.code, key_event.modifiers) {
                    break;
                }
            }
        }
    }

    Ok(())
}
