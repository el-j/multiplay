//! CQRS command-event bus coordinator.
//! Unbounded MPSC channel for inbound commands and broadcast channel for outbound events.

#![forbid(unsafe_code)]

use super::events::{PlayerCommand, PlayerEvent};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct EventBus {
    pub cmd_sender: mpsc::UnboundedSender<PlayerCommand>,
    pub event_broadcaster: broadcast::Sender<PlayerEvent>,
}

impl EventBus {
    /// Initializes a new command queue and event broadcaster with specified subscriber buffer capacity.
    pub fn new(capacity: usize) -> (Self, mpsc::UnboundedReceiver<PlayerCommand>) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (evt_tx, _) = broadcast::channel(capacity);

        (
            Self {
                cmd_sender: cmd_tx,
                event_broadcaster: evt_tx,
            },
            cmd_rx,
        )
    }

    /// Subscribes an observer (e.g. TUI renderer or status logger) to the outbound event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<PlayerEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Sends a player command from a UI control into the engine command loop.
    pub fn send_command(&self, cmd: PlayerCommand) {
        let _ = self.cmd_sender.send(cmd);
    }

    /// Emits a state change event to all active observers.
    pub fn emit_event(&self, event: PlayerEvent) {
        log_event(&event);
        let _ = self.event_broadcaster.send(event);
    }
}

/// The events worth a line in the log file (TASK-177); per-frame ones (position, spectrum) are not.
fn log_event(event: &PlayerEvent) {
    match event {
        PlayerEvent::Error {
            kind,
            source,
            message,
            recoverable,
        } => tracing::warn!(?kind, ?source, recoverable, "playback error: {message}"),
        PlayerEvent::TrackStarted(track) => {
            tracing::info!(source = ?track.source, "track started: {}", track.metadata.title);
        }
        PlayerEvent::PlaybackStateChanged(state) => tracing::debug!(?state, "playback state"),
        PlayerEvent::BufferHealth { .. } => tracing::trace!(?event, "buffer health"),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::PlaybackState;

    #[tokio::test]
    async fn test_cqrs_bus_dispatch_and_broadcast() {
        let (bus, mut cmd_rx) = EventBus::new(16);
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        // 1. Send command
        bus.send_command(PlayerCommand::TogglePause);
        let cmd = cmd_rx.recv().await.expect("receive command failed");
        assert!(matches!(cmd, PlayerCommand::TogglePause));

        // 2. Broadcast event
        bus.emit_event(PlayerEvent::PlaybackStateChanged(PlaybackState::Playing));

        let evt1 = sub1.recv().await.expect("sub1 recv failed");
        let evt2 = sub2.recv().await.expect("sub2 recv failed");

        assert!(matches!(
            evt1,
            PlayerEvent::PlaybackStateChanged(PlaybackState::Playing)
        ));
        assert!(matches!(
            evt2,
            PlayerEvent::PlaybackStateChanged(PlaybackState::Playing)
        ));
    }
}
