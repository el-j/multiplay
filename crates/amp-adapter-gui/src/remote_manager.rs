//! Remote daemon lifecycle and QR pairing coordinator for native GUI adapter.

#![forbid(unsafe_code)]

use amp_adapter_remote::pairing::{PairingManager, QrPairingRenderer};
use amp_adapter_remote::server::RemoteDaemon;
use amp_adapter_remote::protocol::RemoteEventBroadcast;
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use amp_domain::Track;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// GUI Remote Control State and Daemon Coordinator.
pub struct RemoteGuiManager {
    pub port: u16,
    pub lan_ip: String,
    pairing_manager: PairingManager,
    pub current_token: String,
    pub qr_matrix: Option<Vec<Vec<bool>>>,
    pub active_peers: Arc<AtomicUsize>,
    daemon: Option<Arc<RemoteDaemon>>,
    pub event_bus: Arc<EventBus>,
    cmd_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
    pub is_running: bool,
}

impl RemoteGuiManager {
    /// Creates a new RemoteGuiManager instance with initial pairing token and QR matrix.
    #[must_use]
    pub fn new() -> Self {
        let (bus, cmd_rx) = EventBus::new(64);
        let event_bus = Arc::new(bus);
        let mut pairing_manager = PairingManager::new();
        let current_token = pairing_manager.generate_token(600); // 10 minutes TTL
        let lan_ip = detect_local_ip();
        let port = 9002;

        let pairing_url = format!("http://{}:{}/?token={}", lan_ip, port, current_token);
        let qr_matrix = QrPairingRenderer::render_matrix(&pairing_url).ok();

        Self {
            port,
            lan_ip,
            pairing_manager,
            current_token,
            qr_matrix,
            active_peers: Arc::new(AtomicUsize::new(0)),
            daemon: None,
            event_bus,
            cmd_rx: Arc::new(Mutex::new(cmd_rx)),
            is_running: false,
        }
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
        format!("http://{}:{}/?token={}", self.lan_ip, self.port, self.current_token)
    }

    /// Refreshes the ephemeral pairing token and updates QR matrix.
    pub fn refresh_token(&mut self) {
        self.current_token = self.pairing_manager.generate_token(600);
        let url = self.pairing_url();
        self.qr_matrix = QrPairingRenderer::render_matrix(&url).ok();
    }

    /// Starts the background HTTP / WebSocket remote daemon.
    pub fn start_daemon(&mut self) {
        if self.is_running {
            return;
        }

        let bind_addr = format!("0.0.0.0:{}", self.port);
        let daemon = Arc::new(RemoteDaemon::new(bind_addr, Arc::clone(&self.event_bus)));
        self.daemon = Some(Arc::clone(&daemon));
        self.is_running = true;

        let daemon_runner = daemon;
        std::thread::Builder::new()
            .name("multiplay-remote-daemon".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    rt.block_on(async move {
                        let _ = daemon_runner.run().await;
                    });
                }
            })
            .ok();
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
                current_track: current_track.map(Box::new),
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

impl Default for RemoteGuiManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Fallback-safe local IP detection without network roundtrips.
fn detect_local_ip() -> String {
    // Attempt to determine local outbound interface address
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                return local_addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_gui_manager_initialization() {
        let mut mgr = RemoteGuiManager::new();
        assert_eq!(mgr.port, 9002);
        assert!(!mgr.current_token.is_empty());
        assert!(mgr.qr_matrix.is_some());

        let url = mgr.pairing_url();
        assert!(url.contains("http://"));
        assert!(url.contains("?token="));

        let old_token = mgr.current_token.clone();
        mgr.refresh_token();
        assert_ne!(mgr.current_token, old_token);
    }
}
