//! Pure-Rust WebSocket daemon hosting remote control connections and broadcasting state.

#![forbid(unsafe_code)]

use crate::protocol::{RemoteAction, RemoteEventBroadcast};
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

/// Embedded WebSocket remote daemon coordinator.
pub struct RemoteDaemon {
    bind_addr: String,
    event_bus: Arc<EventBus>,
    active_peers: Arc<AtomicUsize>,
    broadcast_tx: broadcast::Sender<RemoteEventBroadcast>,
}

impl RemoteDaemon {
    /// Creates a new RemoteDaemon instance bound to an address and attached to the engine EventBus.
    #[must_use]
    pub fn new(bind_addr: impl Into<String>, event_bus: Arc<EventBus>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        Self {
            bind_addr: bind_addr.into(),
            event_bus,
            active_peers: Arc::new(AtomicUsize::new(0)),
            broadcast_tx,
        }
    }

    /// Returns the number of currently connected peer remotes.
    #[must_use]
    pub fn active_peers_count(&self) -> usize {
        self.active_peers.load(Ordering::SeqCst)
    }

    /// Broadcasts an event to all connected remote clients.
    pub fn broadcast(&self, event: RemoteEventBroadcast) {
        let _ = self.broadcast_tx.send(event);
    }

    /// Starts the listener loop, accepting incoming WebSocket connections.
    pub async fn run(&self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(&self.bind_addr).await?;

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let event_bus = Arc::clone(&self.event_bus);
            let active_peers = Arc::clone(&self.active_peers);
            let broadcast_rx = self.broadcast_tx.subscribe();

            tokio::spawn(async move {
                Self::handle_connection(stream, peer_addr, event_bus, active_peers, broadcast_rx).await;
            });
        }
    }

    /// Handles a single upgraded WebSocket client session.
    async fn handle_connection(
        stream: TcpStream,
        _peer_addr: SocketAddr,
        event_bus: Arc<EventBus>,
        active_peers: Arc<AtomicUsize>,
        mut broadcast_rx: broadcast::Receiver<RemoteEventBroadcast>,
    ) {
        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(_) => return,
        };

        active_peers.fetch_add(1, Ordering::SeqCst);
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        loop {
            tokio::select! {
                Some(msg_res) = ws_receiver.next() => {
                    match msg_res {
                        Ok(Message::Text(text)) => {
                            if let Ok(action) = serde_json::from_str::<RemoteAction>(&text) {
                                Self::dispatch_action(action, &event_bus);
                            }
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
                Ok(event) = broadcast_rx.recv() => {
                    if let Ok(json) = serde_json::to_string(&event) {
                        if ws_sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
                else => break,
            }
        }

        active_peers.fetch_sub(1, Ordering::SeqCst);
    }

    /// Translates a high-level `RemoteAction` into an internal `PlayerCommand`.
    pub fn dispatch_action(action: RemoteAction, event_bus: &EventBus) {
        match action {
            RemoteAction::Play | RemoteAction::Pause => {
                event_bus.send_command(PlayerCommand::TogglePause);
            }
            RemoteAction::Next => {
                event_bus.send_command(PlayerCommand::SeekRelative(Duration::from_secs(10), true));
            }
            RemoteAction::Previous => {
                event_bus.send_command(PlayerCommand::SeekRelative(Duration::from_secs(10), false));
            }
            RemoteAction::SetVolume(vol) => {
                event_bus.send_command(PlayerCommand::SetVolume(vol.clamp(0.0, 1.0)));
            }
            RemoteAction::Seek(secs) => {
                event_bus.send_command(PlayerCommand::SeekAbsolute(Duration::from_secs(secs)));
            }
            RemoteAction::EnqueueTrack(track) => {
                event_bus.send_command(PlayerCommand::AddToQueue(track));
            }
            RemoteAction::VoteSkip | RemoteAction::GetPlaybackStatus => {
                // Handled in collaborative context
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_action_dispatch() {
        let (bus, mut cmd_rx) = EventBus::new(16);
        let arc_bus = Arc::new(bus);

        RemoteDaemon::dispatch_action(RemoteAction::Play, &arc_bus);
        assert!(matches!(cmd_rx.try_recv(), Ok(PlayerCommand::TogglePause)));

        RemoteDaemon::dispatch_action(RemoteAction::SetVolume(0.75), &arc_bus);
        assert!(matches!(cmd_rx.try_recv(), Ok(PlayerCommand::SetVolume(v)) if (v - 0.75).abs() < f32::EPSILON));
    }
}
