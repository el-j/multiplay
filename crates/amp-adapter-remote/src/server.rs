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
use tokio::io::AsyncWriteExt;
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
                Self::handle_connection(stream, peer_addr, event_bus, active_peers, broadcast_rx)
                    .await;
            });
        }
    }

    /// Handles a single upgraded WebSocket client session or HTTP web client request.
    async fn handle_connection(
        mut stream: TcpStream,
        _peer_addr: SocketAddr,
        event_bus: Arc<EventBus>,
        active_peers: Arc<AtomicUsize>,
        mut broadcast_rx: broadcast::Receiver<RemoteEventBroadcast>,
    ) {
        // Peek incoming HTTP request to check if it's an HTTP GET or WebSocket upgrade
        let mut peek_buf = [0u8; 1024];
        let peeked = stream.peek(&mut peek_buf).await.unwrap_or(0);
        let req_text = String::from_utf8_lossy(&peek_buf[..peeked]);
        let is_ws_upgrade = req_text.to_lowercase().contains("upgrade: websocket");

        if !is_ws_upgrade
            && (req_text.starts_with("GET / ")
                || req_text.starts_with("GET /?")
                || req_text.starts_with("GET /index.html"))
        {
            let html = crate::web_client::MOBILE_WEB_CLIENT_HTML;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.flush().await;
            return;
        }

        if !is_ws_upgrade && req_text.starts_with("GET /api/status") {
            let body = "{\"status\":\"ok\",\"service\":\"multiplay-remote\"}";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.flush().await;
            return;
        }

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
            RemoteAction::VoteSkip => {
                event_bus.send_command(PlayerCommand::SeekRelative(Duration::ZERO, true));
            }
            RemoteAction::GetPlaybackStatus => {}
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
        assert!(
            matches!(cmd_rx.try_recv(), Ok(PlayerCommand::SetVolume(v)) if (v - 0.75).abs() < f32::EPSILON)
        );
    }

    #[tokio::test]
    async fn test_remote_daemon_http_web_client_serving() {
        let (bus, _) = EventBus::new(16);
        let arc_bus = Arc::new(bus);

        // Bind ephemeral port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let bind_addr = format!("127.0.0.1:{port}");
        let daemon = Arc::new(RemoteDaemon::new(&bind_addr, arc_bus));
        let daemon_runner = daemon.clone();

        tokio::spawn(async move {
            let _ = daemon_runner.run().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut client = tokio::net::TcpStream::connect(&bind_addr).await.expect("connect");
        client.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await.unwrap();

        let mut resp = vec![0u8; 4096];
        let n = client.read(&mut resp).await.expect("read response");
        let resp_str = String::from_utf8_lossy(&resp[..n]);

        assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
        assert!(resp_str.contains("MULTIPLAY REMOTE"));
    }

    #[test]
    fn test_remote_collaborative_vote_skip() {
        let (bus, mut cmd_rx) = EventBus::new(16);
        let arc_bus = Arc::new(bus);

        RemoteDaemon::dispatch_action(RemoteAction::VoteSkip, &arc_bus);
        assert!(matches!(
            cmd_rx.try_recv(),
            Ok(PlayerCommand::SeekRelative(_, true))
        ));
    }

    #[test]
    fn test_remote_collaborative_enqueue() {
        let (bus, mut cmd_rx) = EventBus::new(16);
        let arc_bus = Arc::new(bus);

        let track = amp_domain::Track::new(
            amp_domain::AudioSource::LocalFile { path: "/song.mp3".into() },
            amp_domain::TrackMetadata::new("Jam Track", "Jam Artist"),
        );

        RemoteDaemon::dispatch_action(RemoteAction::EnqueueTrack(Box::new(track)), &arc_bus);
        assert!(matches!(cmd_rx.try_recv(), Ok(PlayerCommand::AddToQueue(_))));
    }
}
