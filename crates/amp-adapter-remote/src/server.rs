//! Embedded WebSocket remote-control daemon.
//!
//! Security model (audit SEC-01..03):
//! - Binds loopback unless the caller explicitly asks for LAN ([`RemoteDaemonConfig::lan`]).
//! - Every upgrade needs a single-use pairing token or a session secret ([`crate::auth`]).
//! - Host must be an IP literal or localhost; Origin must match it ([`crate::handshake`]).
//! - Failed attempts are throttled per address; peers, frame size and handshake time are capped.

#![forbid(unsafe_code)]

use crate::auth::{FailureThrottle, PairingTokens, SessionStore};
use crate::handshake::{self, Credential, Rejection};
use crate::protocol::{self, RemoteAction, RemoteEventBroadcast};
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use futures_util::{SinkExt, StreamExt};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, watch};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::header;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

/// Default TCP port of the remote daemon.
pub const DEFAULT_PORT: u16 = 9002;

/// Runtime limits and bind address of the remote daemon.
#[derive(Debug, Clone)]
pub struct RemoteDaemonConfig {
    /// Socket address to listen on.
    pub bind_addr: SocketAddr,
    /// Maximum simultaneously connected remotes.
    pub max_peers: usize,
    /// Maximum size of one WebSocket message or frame, in bytes.
    pub max_message_bytes: usize,
    /// Time allowed for a client to finish the HTTP/WebSocket handshake.
    pub handshake_timeout: Duration,
    /// Where hashed session secrets are persisted; `None` keeps them in memory.
    pub session_store: Option<PathBuf>,
}

impl RemoteDaemonConfig {
    /// Listens on `127.0.0.1:port` only: reachable from this machine, not the network.
    #[must_use]
    pub fn loopback(port: u16) -> Self {
        Self::with_ip(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    /// Listens on all interfaces so phones on the same network can connect.
    #[must_use]
    pub fn lan(port: u16) -> Self {
        Self::with_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
    }

    fn with_ip(ip: IpAddr, port: u16) -> Self {
        Self {
            bind_addr: SocketAddr::new(ip, port),
            max_peers: 8,
            max_message_bytes: 64 * 1024,
            handshake_timeout: Duration::from_secs(10),
            session_store: None,
        }
    }
}

struct AuthState {
    tokens: PairingTokens,
    sessions: SessionStore,
    throttle: FailureThrottle,
}

struct Shared {
    config: RemoteDaemonConfig,
    event_bus: Arc<EventBus>,
    active_peers: Arc<AtomicUsize>,
    broadcast_tx: broadcast::Sender<RemoteEventBroadcast>,
    auth: Mutex<AuthState>,
    shutdown_tx: watch::Sender<bool>,
}

/// Embedded WebSocket remote daemon.
pub struct RemoteDaemon {
    shared: Arc<Shared>,
}

impl RemoteDaemon {
    /// Creates a daemon attached to the engine [`EventBus`]. Nothing listens until [`Self::run`].
    #[must_use]
    pub fn new(config: RemoteDaemonConfig, event_bus: Arc<EventBus>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        let (shutdown_tx, _) = watch::channel(false);
        let sessions = SessionStore::load(config.session_store.clone());
        Self {
            shared: Arc::new(Shared {
                config,
                event_bus,
                active_peers: Arc::new(AtomicUsize::new(0)),
                broadcast_tx,
                auth: Mutex::new(AuthState {
                    tokens: PairingTokens::default(),
                    sessions,
                    throttle: FailureThrottle::default(),
                }),
                shutdown_tx,
            }),
        }
    }

    /// Issues a single-use pairing token valid for `ttl` (shown to the user as a QR code).
    pub fn issue_pairing_token(&self, ttl: Duration) -> String {
        lock(&self.shared.auth).tokens.issue(ttl)
    }

    /// Revokes every paired device.
    pub fn revoke_all_sessions(&self) {
        lock(&self.shared.auth).sessions.revoke_all();
    }

    /// Number of devices holding a valid session secret.
    #[must_use]
    pub fn paired_devices(&self) -> usize {
        lock(&self.shared.auth).sessions.len()
    }

    /// Returns the number of currently connected remotes.
    #[must_use]
    pub fn active_peers_count(&self) -> usize {
        self.shared.active_peers.load(Ordering::SeqCst)
    }

    /// Broadcasts an event to all connected remotes.
    pub fn broadcast(&self, event: RemoteEventBroadcast) {
        let _ = self.shared.broadcast_tx.send(event);
    }

    /// Stops accepting connections and closes all sessions.
    pub fn shutdown(&self) {
        self.shared.shutdown_tx.send_replace(true);
    }

    /// Binds the configured address and serves until [`Self::shutdown`].
    pub async fn run(&self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(self.shared.config.bind_addr).await?;
        self.run_with_listener(listener).await
    }

    /// Serves on an already-bound listener until [`Self::shutdown`].
    pub async fn run_with_listener(&self, listener: TcpListener) -> Result<(), std::io::Error> {
        let mut shutdown_rx = self.shared.shutdown_tx.subscribe();
        if *shutdown_rx.borrow() {
            return Ok(());
        }
        loop {
            tokio::select! {
                accepted = listener.accept() => match accepted {
                    Ok((stream, peer)) => {
                        let shared = Arc::clone(&self.shared);
                        tokio::spawn(async move { handle_connection(shared, stream, peer).await });
                    }
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                },
                _ = shutdown_rx.changed() => return Ok(()),
            }
        }
    }

    /// Translates a remote action into an engine command. Unknown or unsafe input never reaches here.
    pub fn dispatch_action(action: RemoteAction, event_bus: &EventBus) {
        let cmd = match action {
            RemoteAction::Play | RemoteAction::Pause => PlayerCommand::TogglePause,
            RemoteAction::Next | RemoteAction::VoteSkip => PlayerCommand::Next,
            RemoteAction::Previous => PlayerCommand::Previous,
            RemoteAction::SetVolume(vol) => PlayerCommand::SetVolume(vol.clamp(0.0, 1.0)),
            RemoteAction::Seek(secs) => PlayerCommand::SeekAbsolute(Duration::from_secs(secs)),
            RemoteAction::EnqueueById(id) => PlayerCommand::EnqueueExisting(id),
            RemoteAction::GetPlaybackStatus => return,
        };
        event_bus.send_command(cmd);
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Outcome of a successful handshake check.
enum Granted {
    /// Paired with a token: the client must receive its new session secret.
    NewSession(String),
    /// Reconnected with an existing session secret.
    Resumed,
}

fn authorize(shared: &Shared, ip: IpAddr, req: &Request) -> Result<Granted, Rejection> {
    let now = Instant::now();
    let mut auth = lock(&shared.auth);
    auth.throttle.check(ip, now).map_err(Rejection::Throttled)?;

    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .map(|o| o.to_str().unwrap_or("invalid"));

    let verdict = if req.uri().path() != "/ws" {
        Err(Rejection::NotFound)
    } else if !handshake::host_is_acceptable(host) {
        Err(Rejection::ForbiddenHost)
    } else if !handshake::origin_matches_host(origin, host) {
        Err(Rejection::ForbiddenOrigin)
    } else if shared.active_peers.load(Ordering::SeqCst) >= shared.config.max_peers {
        return Err(Rejection::Busy);
    } else {
        match handshake::credential_from_query(req.uri().query()) {
            Some(Credential::PairingToken(t)) if auth.tokens.consume(&t) => {
                Ok(Granted::NewSession(auth.sessions.issue()))
            }
            Some(Credential::Session(s)) if auth.sessions.verify(&s) => Ok(Granted::Resumed),
            _ => Err(Rejection::Unauthorized),
        }
    };
    match &verdict {
        Ok(_) => auth.throttle.record_success(ip),
        Err(_) => auth.throttle.record_failure(ip, now),
    }
    verdict
}

// tungstenite's handshake callback signature fixes the large `ErrorResponse` error type.
#[allow(clippy::result_large_err)]
async fn handle_connection(shared: Arc<Shared>, mut stream: TcpStream, peer: SocketAddr) {
    let timeout = shared.config.handshake_timeout;
    let mut peek_buf = [0u8; 2048];
    let peeked = match tokio::time::timeout(timeout, stream.peek(&mut peek_buf)).await {
        Ok(Ok(n)) => n,
        _ => return,
    };
    let head = String::from_utf8_lossy(&peek_buf[..peeked]);
    if !head.to_ascii_lowercase().contains("upgrade: websocket") {
        let response = crate::http::respond(head.lines().next().unwrap_or(""));
        // Drain the peeked request so closing the socket does not send a TCP reset.
        let mut drain = [0u8; 2048];
        let _ = tokio::time::timeout(timeout, stream.read(&mut drain[..peeked])).await;
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
        return;
    }

    let mut granted = None;
    let callback = |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
        match authorize(&shared, peer.ip(), req) {
            Ok(g) => {
                granted = Some(g);
                Ok(resp)
            }
            Err(rejection) => Err(rejection.into_response()),
        }
    };
    let ws_config = WebSocketConfig::default()
        .max_message_size(Some(shared.config.max_message_bytes))
        .max_frame_size(Some(shared.config.max_message_bytes));
    let accept = tokio_tungstenite::accept_hdr_async_with_config(stream, callback, Some(ws_config));
    let ws = match tokio::time::timeout(timeout, accept).await {
        Ok(Ok(ws)) => ws,
        _ => return,
    };

    shared.active_peers.fetch_add(1, Ordering::SeqCst);
    let (mut tx, mut rx) = ws.split();
    let mut broadcast_rx = shared.broadcast_tx.subscribe();
    let mut shutdown_rx = shared.shutdown_tx.subscribe();

    if let Some(Granted::NewSession(session)) = granted {
        let msg =
            protocol::encode_server_message(&RemoteEventBroadcast::SessionGranted { session });
        if tx.send(Message::Text(msg.into())).await.is_err() {
            shared.active_peers.fetch_sub(1, Ordering::SeqCst);
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = rx.next() => match incoming {
                Some(Ok(Message::Text(text))) => match protocol::parse_client_message(&text) {
                    Ok(action) => RemoteDaemon::dispatch_action(action, &shared.event_bus),
                    Err(e) => {
                        let err = RemoteEventBroadcast::Error { message: e.to_string() };
                        if tx.send(Message::Text(protocol::encode_server_message(&err).into())).await.is_err() {
                            break;
                        }
                    }
                },
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
            event = broadcast_rx.recv() => match event {
                Ok(event) => {
                    if tx.send(Message::Text(protocol::encode_server_message(&event).into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            },
            _ = shutdown_rx.changed() => {
                let _ = tx.send(Message::Close(None)).await;
                break;
            }
        }
    }
    shared.active_peers.fetch_sub(1, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_is_the_default_bind() {
        assert!(RemoteDaemonConfig::loopback(DEFAULT_PORT)
            .bind_addr
            .ip()
            .is_loopback());
        assert!(RemoteDaemonConfig::lan(DEFAULT_PORT)
            .bind_addr
            .ip()
            .is_unspecified());
    }

    #[test]
    fn actions_map_to_real_transport_commands() {
        let (bus, mut rx) = EventBus::new(16);
        RemoteDaemon::dispatch_action(RemoteAction::Next, &bus);
        assert!(matches!(rx.try_recv(), Ok(PlayerCommand::Next)));
        RemoteDaemon::dispatch_action(RemoteAction::VoteSkip, &bus);
        assert!(matches!(rx.try_recv(), Ok(PlayerCommand::Next)));
        RemoteDaemon::dispatch_action(RemoteAction::Previous, &bus);
        assert!(matches!(rx.try_recv(), Ok(PlayerCommand::Previous)));
        RemoteDaemon::dispatch_action(RemoteAction::SetVolume(3.0), &bus);
        assert!(matches!(rx.try_recv(), Ok(PlayerCommand::SetVolume(v)) if v == 1.0));
        RemoteDaemon::dispatch_action(RemoteAction::GetPlaybackStatus, &bus);
        assert!(rx.try_recv().is_err());
    }
}
