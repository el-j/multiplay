//! Embedded WebSocket remote-control daemon.
//!
//! Security model (audit SEC-01..03, TASK-215):
//! - Binds loopback unless the caller explicitly asks for LAN ([`RemoteDaemonConfig::lan`]).
//! - Every upgrade needs a single-use pairing token or a device's session secret.
//! - Host must be an IP literal or localhost; Origin must match it ([`crate::handshake`]).
//! - Failed attempts are throttled per address; peers, frame size and handshake time are capped.
//! - Track requests are validated references, rate limited per device, and only offered to the
//!   host ([`HostRequest`]); nothing a phone sends opens or plays anything by itself.

#![forbid(unsafe_code)]

use crate::auth::{FailureThrottle, PairingTokens};
use crate::connection;
use crate::devices::{device_label, DeviceId, DeviceSummary, SessionStore};
use crate::handshake::{self, Credential, Rejection};
use crate::protocol::{RemoteAction, RemoteEventBroadcast};
use crate::reference::SearchItem;
use crate::requests::{HostRequest, RequestBook, RequestId, RequestStatus};
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::http::header;

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
    /// Folder for opt-in phone uploads (TASK-218); `None` never accepts uploads.
    pub upload_dir: Option<PathBuf>,
    /// Largest accepted upload, in bytes.
    pub max_upload_bytes: u64,
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
            upload_dir: None,
            max_upload_bytes: crate::upload::DEFAULT_MAX_UPLOAD_BYTES,
        }
    }
}

pub(crate) struct AuthState {
    pub(crate) tokens: PairingTokens,
    pub(crate) sessions: SessionStore,
    pub(crate) throttle: FailureThrottle,
}

/// What the daemon pushes to open sockets.
#[derive(Debug, Clone)]
pub(crate) enum Outbound {
    /// To every connected phone.
    All(RemoteEventBroadcast),
    /// To the sockets of one device only.
    To(DeviceId, RemoteEventBroadcast),
    /// Close the sockets of one device, or of every device.
    Kick(Option<DeviceId>),
}

pub(crate) struct Shared {
    pub(crate) config: RemoteDaemonConfig,
    pub(crate) event_bus: Arc<EventBus>,
    pub(crate) active_peers: Arc<AtomicUsize>,
    pub(crate) outbound: broadcast::Sender<Outbound>,
    pub(crate) auth: Mutex<AuthState>,
    pub(crate) requests: Mutex<RequestBook>,
    pub(crate) host_tx: mpsc::UnboundedSender<HostRequest>,
    /// Open sockets per device.
    pub(crate) connected: Mutex<HashMap<DeviceId, usize>>,
    pub(crate) uploads: crate::upload::Uploads,
    pub(crate) shutdown_tx: watch::Sender<bool>,
}

impl Shared {
    /// Uploads are on, and there is somewhere to put them and a way to check them.
    pub(crate) fn uploads_available(&self) -> bool {
        self.uploads.enabled.load(Ordering::SeqCst)
            && self.config.upload_dir.is_some()
            && lock(&self.uploads.probe).is_some()
    }
}

/// Embedded WebSocket remote daemon.
pub struct RemoteDaemon {
    shared: Arc<Shared>,
    host_rx: Mutex<Option<mpsc::UnboundedReceiver<HostRequest>>>,
}

impl RemoteDaemon {
    /// Creates a daemon attached to the engine [`EventBus`]. Nothing listens until [`Self::run`].
    #[must_use]
    pub fn new(config: RemoteDaemonConfig, event_bus: Arc<EventBus>) -> Self {
        let (outbound, _) = broadcast::channel(64);
        let (shutdown_tx, _) = watch::channel(false);
        let (host_tx, host_rx) = mpsc::unbounded_channel();
        let sessions = SessionStore::load(config.session_store.clone());
        if let Some(dir) = &config.upload_dir {
            crate::upload::clear(dir);
        }
        Self {
            shared: Arc::new(Shared {
                config,
                event_bus,
                active_peers: Arc::new(AtomicUsize::new(0)),
                outbound,
                auth: Mutex::new(AuthState {
                    tokens: PairingTokens::default(),
                    sessions,
                    throttle: FailureThrottle::default(),
                }),
                requests: Mutex::new(RequestBook::default()),
                host_tx,
                connected: Mutex::new(HashMap::new()),
                uploads: crate::upload::Uploads::default(),
                shutdown_tx,
            }),
            host_rx: Mutex::new(Some(host_rx)),
        }
    }

    /// The stream of track requests and searches the host must answer. Handed out once.
    pub fn take_request_receiver(&self) -> Option<mpsc::UnboundedReceiver<HostRequest>> {
        lock(&self.host_rx).take()
    }

    /// Issues a single-use pairing token valid for `ttl` (shown to the user as a QR code).
    pub fn issue_pairing_token(&self, ttl: Duration) -> String {
        lock(&self.shared.auth).tokens.issue(ttl)
    }

    /// Revokes every paired device and disconnects them.
    pub fn revoke_all_sessions(&self) {
        lock(&self.shared.auth).sessions.revoke_all();
        let _ = self.shared.outbound.send(Outbound::Kick(None));
    }

    /// Revokes one device and closes its sockets; others stay connected. False if unknown.
    pub fn revoke_device(&self, id: &DeviceId) -> bool {
        let removed = lock(&self.shared.auth).sessions.revoke(id);
        if removed {
            lock(&self.shared.requests).forget_device(id);
            let _ = self.shared.outbound.send(Outbound::Kick(Some(id.clone())));
        }
        removed
    }

    /// Sets whether `id`'s requests are accepted without asking. False if unknown.
    pub fn set_trusted(&self, id: &DeviceId, trusted: bool) -> bool {
        lock(&self.shared.auth).sessions.set_trusted(id, trusted)
    }

    /// Paired devices, oldest first, with whether each is connected now.
    #[must_use]
    pub fn devices(&self) -> Vec<DeviceSummary> {
        let mut list = lock(&self.shared.auth).sessions.list();
        let connected = lock(&self.shared.connected);
        for d in &mut list {
            d.connected = connected.get(&d.id).is_some_and(|n| *n > 0);
        }
        list
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
        let _ = self.shared.outbound.send(Outbound::All(event));
    }

    /// Tells the requesting phone what happened to request `id`.
    pub fn update_request(&self, id: RequestId, status: RequestStatus, title: Option<String>) {
        if let Some(device) = lock(&self.shared.requests).update(id, status) {
            let event = RemoteEventBroadcast::RequestUpdate { id, status, title };
            let _ = self.shared.outbound.send(Outbound::To(device, event));
        }
    }

    /// Answers a [`HostRequest::Search`] from `device`.
    pub fn send_search_results(&self, device: &DeviceId, query: &str, items: Vec<SearchItem>) {
        let event = RemoteEventBroadcast::SearchResults {
            query: query.to_string(),
            items,
        };
        let _ = self
            .shared
            .outbound
            .send(Outbound::To(device.clone(), event));
    }

    /// Sets how uploaded files are checked before the host sees them (a decode probe).
    pub fn set_upload_probe(&self, probe: crate::upload::UploadProbe) {
        *lock(&self.shared.uploads.probe) = Some(probe);
    }

    /// Lets paired phones upload audio files for this session, or stops it. Off by default;
    /// uploads also need an upload folder and a probe.
    pub fn set_uploads_enabled(&self, enabled: bool) {
        self.shared.uploads.enabled.store(enabled, Ordering::SeqCst);
        let enabled = self.uploads_enabled();
        let _ = self
            .shared
            .outbound
            .send(Outbound::All(RemoteEventBroadcast::Uploads { enabled }));
    }

    /// True when phones can upload right now.
    #[must_use]
    pub fn uploads_enabled(&self) -> bool {
        self.shared.uploads_available()
    }

    /// Stops accepting connections, closes all sessions and deletes uploaded files.
    pub fn shutdown(&self) {
        self.shared.shutdown_tx.send_replace(true);
        if let Some(dir) = &self.shared.config.upload_dir {
            crate::upload::clear(dir);
        }
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
                        tokio::spawn(async move {
                            connection::handle_connection(shared, stream, peer).await
                        });
                    }
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                },
                _ = shutdown_rx.changed() => return Ok(()),
            }
        }
    }

    /// Translates a transport action into an engine command. Requests and searches are not
    /// transport actions; they go to the host for moderation instead.
    pub fn dispatch_action(action: RemoteAction, event_bus: &EventBus) {
        let cmd = match action {
            RemoteAction::Play | RemoteAction::Pause => PlayerCommand::TogglePause,
            RemoteAction::Next | RemoteAction::VoteSkip => PlayerCommand::Next,
            RemoteAction::Previous => PlayerCommand::Previous,
            RemoteAction::SetVolume(vol) => PlayerCommand::SetVolume(vol.clamp(0.0, 1.0)),
            RemoteAction::Seek(secs) => PlayerCommand::SeekAbsolute(Duration::from_secs(secs)),
            RemoteAction::GetPlaybackStatus
            | RemoteAction::EnqueueById(_)
            | RemoteAction::RequestTrack(_)
            | RemoteAction::Search(_) => return,
        };
        event_bus.send_command(cmd);
    }
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Outcome of a successful handshake check.
pub(crate) enum Granted {
    /// Paired with a token: the client must receive its new session secret.
    NewSession(DeviceId, String),
    /// Reconnected with an existing session secret.
    Resumed(DeviceId),
}

impl Granted {
    pub(crate) fn device(&self) -> &DeviceId {
        match self {
            Self::NewSession(id, _) | Self::Resumed(id) => id,
        }
    }
}

pub(crate) fn authorize(shared: &Shared, ip: IpAddr, req: &Request) -> Result<Granted, Rejection> {
    let now = Instant::now();
    let mut auth = lock(&shared.auth);
    auth.throttle.check(ip, now).map_err(Rejection::Throttled)?;

    let header_str = |name| req.headers().get(name).and_then(|h| h.to_str().ok());
    let host = header_str(header::HOST).unwrap_or("");
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
                let name = device_label(header_str(header::USER_AGENT));
                let (id, secret) = auth.sessions.issue(&name);
                Ok(Granted::NewSession(id, secret))
            }
            Some(Credential::Session(s)) => match auth.sessions.verify(&s) {
                Some(id) => Ok(Granted::Resumed(id)),
                None => Err(Rejection::Unauthorized),
            },
            _ => Err(Rejection::Unauthorized),
        }
    };
    match &verdict {
        Ok(_) => auth.throttle.record_success(ip),
        Err(_) => auth.throttle.record_failure(ip, now),
    }
    verdict
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
        RemoteDaemon::dispatch_action(RemoteAction::EnqueueById(amp_domain::TrackId::new()), &bus);
        assert!(rx.try_recv().is_err(), "requests never become commands");
    }
}
