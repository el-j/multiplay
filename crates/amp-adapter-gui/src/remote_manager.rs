//! Remote daemon lifecycle and QR pairing coordinator for native GUI adapter.
//!
//! The daemon never starts on its own: the user enables it in Settings → Remote.
//! It listens on loopback unless "Allow devices on my network" is also enabled.

#![forbid(unsafe_code)]

use amp_adapter_remote::{
    DeviceId, DeviceSummary, HostRequest, QrPairingRenderer, RemoteDaemon, RemoteDaemonConfig,
    RemoteEventBroadcast, RemoteTrackInfo, RequestId, RequestStatus, SearchItem,
};
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use amp_domain::{RemoteControlConfig, Track};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// Lifetime of a pairing token shown as a QR code.
const PAIRING_TTL: Duration = Duration::from_secs(600);

/// GUI Remote Control State and Daemon Coordinator.
pub struct RemoteGuiManager {
    pub port: u16,
    pub lan_ip: String,
    pub allow_lan: bool,
    pub current_token: String,
    pub qr_matrix: Option<Vec<Vec<bool>>>,
    /// Last start failure (e.g. port in use), shown in Settings.
    pub last_error: Option<String>,
    daemon: Option<Arc<RemoteDaemon>>,
    session_store: Option<PathBuf>,
    pub event_bus: Arc<EventBus>,
    cmd_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
    /// Track requests and searches from phones, for the Remote window.
    requests: Option<mpsc::UnboundedReceiver<HostRequest>>,
    /// Counts daemon starts; request ids are only unique within one run.
    pub generation: u64,
    /// Where phone uploads are kept while the daemon runs; `None` disables uploads.
    pub upload_dir: Option<PathBuf>,
    pub is_running: bool,
}

impl RemoteGuiManager {
    /// Creates a stopped manager. `session_store` persists paired devices (hashed); `None` keeps them in memory.
    #[must_use]
    pub fn new(session_store: Option<PathBuf>) -> Self {
        let (bus, cmd_rx) = EventBus::new(64);
        Self {
            port: RemoteControlConfig::default().port,
            lan_ip: Ipv4Addr::LOCALHOST.to_string(),
            allow_lan: false,
            current_token: String::new(),
            qr_matrix: None,
            last_error: None,
            daemon: None,
            session_store,
            event_bus: Arc::new(bus),
            cmd_rx: Arc::new(Mutex::new(cmd_rx)),
            requests: None,
            generation: 0,
            upload_dir: None,
            is_running: false,
        }
    }

    /// Next track request or search from a phone, if any.
    pub fn poll_request(&mut self) -> Option<HostRequest> {
        self.requests.as_mut()?.try_recv().ok()
    }

    /// Tells the requesting phone what happened to its request.
    pub fn update_request(&self, id: RequestId, status: RequestStatus, title: Option<String>) {
        if let Some(d) = &self.daemon {
            d.update_request(id, status, title);
        }
    }

    /// Answers a phone's search.
    pub fn send_search_results(&self, device: &DeviceId, query: &str, items: Vec<SearchItem>) {
        if let Some(d) = &self.daemon {
            d.send_search_results(device, query, items);
        }
    }

    /// True while phones may upload audio files (off at every start).
    #[must_use]
    pub fn uploads_enabled(&self) -> bool {
        self.daemon.as_ref().is_some_and(|d| d.uploads_enabled())
    }

    /// Lets phones upload audio files until the daemon stops.
    pub fn set_uploads_enabled(&self, enabled: bool) {
        if let Some(d) = &self.daemon {
            d.set_uploads_enabled(enabled);
        }
    }

    /// Paired devices, oldest first (empty while the daemon is off).
    #[must_use]
    pub fn devices(&self) -> Vec<DeviceSummary> {
        self.daemon
            .as_ref()
            .map(|d| d.devices())
            .unwrap_or_default()
    }

    /// Forgets one device and disconnects it.
    pub fn revoke_device(&self, id: &DeviceId) -> bool {
        self.daemon.as_ref().is_some_and(|d| d.revoke_device(id))
    }

    /// Accept `id`'s requests without asking (or ask again).
    pub fn set_trusted(&self, id: &DeviceId, trusted: bool) -> bool {
        self.daemon
            .as_ref()
            .is_some_and(|d| d.set_trusted(id, trusted))
    }

    /// Polls for incoming commands sent from mobile remote clients.
    pub fn try_recv_command(&self) -> Option<PlayerCommand> {
        if let Ok(mut rx) = self.cmd_rx.try_lock() {
            rx.try_recv().ok()
        } else {
            None
        }
    }

    /// Full pairing URL for mobile browsers.
    #[must_use]
    pub fn pairing_url(&self) -> String {
        format!(
            "http://{}:{}/?token={}",
            self.lan_ip, self.port, self.current_token
        )
    }

    /// Issues a fresh single-use pairing token and updates the QR matrix.
    pub fn refresh_token(&mut self) {
        let Some(daemon) = &self.daemon else { return };
        self.current_token = daemon.issue_pairing_token(PAIRING_TTL);
        self.qr_matrix = QrPairingRenderer::render_matrix(&self.pairing_url()).ok();
    }

    /// Starts or stops the daemon so it matches `config`.
    pub fn apply_config(&mut self, config: &RemoteControlConfig) {
        let wants_restart =
            self.is_running && (config.allow_lan != self.allow_lan || config.port != self.port);
        if !config.enabled || wants_restart {
            self.stop_daemon();
        }
        self.port = config.port;
        self.allow_lan = config.allow_lan;
        if config.enabled && !self.is_running {
            self.start_daemon();
        }
    }

    /// Starts the background HTTP / WebSocket remote daemon.
    pub fn start_daemon(&mut self) {
        if self.is_running {
            return;
        }
        let mut daemon_config = if self.allow_lan {
            RemoteDaemonConfig::lan(self.port)
        } else {
            RemoteDaemonConfig::loopback(self.port)
        };
        daemon_config.session_store = self.session_store.clone();
        daemon_config.upload_dir = self.upload_dir.clone();
        let bind_addr = daemon_config.bind_addr;

        // Bind synchronously so a busy port is reported instead of silently ignored.
        let listener = match std::net::TcpListener::bind(bind_addr).and_then(|l| {
            l.set_nonblocking(true)?;
            Ok(l)
        }) {
            Ok(l) => l,
            Err(e) => {
                self.last_error = Some(format!("Could not listen on {bind_addr}: {e}"));
                return;
            }
        };

        let daemon = Arc::new(RemoteDaemon::new(
            daemon_config,
            Arc::clone(&self.event_bus),
        ));
        let runner = Arc::clone(&daemon);
        let spawned = std::thread::Builder::new()
            .name("multiplay-remote-daemon".to_string())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                rt.block_on(async move {
                    if let Ok(listener) = tokio::net::TcpListener::from_std(listener) {
                        let _ = runner.run_with_listener(listener).await;
                    }
                });
            });
        if let Err(e) = spawned {
            self.last_error = Some(format!("Could not start remote daemon thread: {e}"));
            return;
        }

        self.lan_ip = if self.allow_lan {
            detect_lan_ip().map_or_else(|| Ipv4Addr::LOCALHOST.to_string(), |ip| ip.to_string())
        } else {
            Ipv4Addr::LOCALHOST.to_string()
        };
        self.requests = daemon.take_request_receiver();
        daemon.set_upload_probe(Arc::new(probe_audio));
        self.daemon = Some(daemon);
        self.generation += 1;
        self.is_running = true;
        self.last_error = None;
        self.refresh_token();
    }

    /// Stops the daemon and disconnects all remotes. Paired devices stay paired.
    pub fn stop_daemon(&mut self) {
        if let Some(daemon) = self.daemon.take() {
            daemon.shutdown();
        }
        self.requests = None;
        self.is_running = false;
        self.current_token.clear();
        self.qr_matrix = None;
    }

    /// Forgets every paired device; they must scan a new QR code.
    pub fn revoke_all_sessions(&self) {
        if let Some(daemon) = &self.daemon {
            daemon.revoke_all_sessions();
        } else if let Some(path) = &self.session_store {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Number of devices currently paired.
    #[must_use]
    pub fn paired_devices(&self) -> usize {
        self.daemon.as_ref().map_or(0, |d| d.paired_devices())
    }

    /// Broadcasts active player status to connected mobile clients.
    pub fn broadcast_status(
        &self,
        current_track: Option<Track>,
        is_playing: bool,
        volume: f32,
        position_secs: u64,
    ) {
        if let Some(ref d) = self.daemon {
            d.broadcast(RemoteEventBroadcast::Status {
                current_track: current_track.as_ref().map(RemoteTrackInfo::from),
                is_playing,
                volume,
                position_secs,
                active_peers: d.active_peers_count(),
            });
        }
    }

    /// Returns currently connected peer count.
    #[must_use]
    pub fn active_peers_count(&self) -> usize {
        self.daemon
            .as_ref()
            .map(|d| d.active_peers_count())
            .unwrap_or(0)
    }
}

impl Drop for RemoteGuiManager {
    fn drop(&mut self) {
        self.stop_daemon();
    }
}

/// Accepts an uploaded file only if the decoder opens it and produces audio.
fn probe_audio(path: &std::path::Path) -> Result<(), String> {
    let mut decoder = amp_adapter_decoder::AudioStreamDecoder::open_file(&path.to_string_lossy())?;
    let mut buf = vec![0.0f32; 4096];
    match decoder.read_interleaved_samples(&mut buf)? {
        0 => Err("no audio in the file".into()),
        _ => Ok(()),
    }
}

/// Picks the address a phone on the same network should use: the first private,
/// non-loopback IPv4 address of an interface that is up. Sends no network traffic.
fn detect_lan_ip() -> Option<IpAddr> {
    let interfaces = if_addrs::get_if_addrs().ok()?;
    let mut candidates: Vec<Ipv4Addr> = interfaces
        .iter()
        .filter(|i| !i.is_loopback() && i.is_oper_up())
        .filter_map(|i| match i.ip() {
            IpAddr::V4(v4) if !v4.is_link_local() => Some(v4),
            _ => None,
        })
        .collect();
    candidates.sort_by_key(|ip| !ip.is_private());
    candidates.first().copied().map(IpAddr::V4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loopback_config(port: u16) -> RemoteControlConfig {
        RemoteControlConfig {
            enabled: true,
            allow_lan: false,
            port,
        }
    }

    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn new_manager_does_not_listen() {
        let mgr = RemoteGuiManager::new(None);
        assert!(!mgr.is_running);
        assert!(mgr.current_token.is_empty());
        assert!(mgr.qr_matrix.is_none());
    }

    #[test]
    fn enabling_binds_loopback_and_disabling_releases_the_port() {
        let port = free_port();
        let mut mgr = RemoteGuiManager::new(None);
        mgr.apply_config(&loopback_config(port));
        assert!(mgr.is_running, "{:?}", mgr.last_error);
        assert_eq!(mgr.lan_ip, "127.0.0.1");
        assert!(mgr.pairing_url().contains("?token="));
        assert!(mgr.qr_matrix.is_some());
        assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_ok());

        let old = mgr.current_token.clone();
        mgr.refresh_token();
        assert_ne!(mgr.current_token, old);

        mgr.apply_config(&RemoteControlConfig::default());
        assert!(!mgr.is_running);
        let released = (0..40).any(|_| {
            std::thread::sleep(Duration::from_millis(50));
            std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
        });
        assert!(released, "port must be released after disabling");
    }

    #[test]
    fn busy_port_is_reported() {
        let blocker = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = blocker.local_addr().unwrap().port();
        let mut mgr = RemoteGuiManager::new(None);
        mgr.apply_config(&loopback_config(port));
        assert!(!mgr.is_running);
        assert!(mgr.last_error.is_some());
    }
}
