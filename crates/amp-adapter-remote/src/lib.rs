//! Pure-Rust embedded WebSocket remote daemon and QR pairing adapter.
//!
//! The daemon is off unless a front end starts it, binds loopback unless LAN access is
//! requested, and only accepts clients that paired with a host-issued token. Remotes never
//! make the host open anything without approval: track requests are handed to the host.

#![forbid(unsafe_code)]

pub mod auth;
mod connection;
pub mod devices;
pub mod handshake;
pub mod http;
pub mod pairing;
pub mod protocol;
pub mod reference;
pub mod requests;
pub mod server;
pub mod upload;
pub mod web_client;

pub use devices::{DeviceId, DeviceSummary};
pub use pairing::QrPairingRenderer;
pub use protocol::{parse_client_message, RemoteAction, RemoteEventBroadcast, RemoteTrackInfo};
pub use reference::{SearchItem, TrackReference, ValidReference};
pub use requests::{HostRequest, RequestId, RequestStatus, TrackRequest, MAX_PENDING_REQUESTS};
pub use server::{RemoteDaemon, RemoteDaemonConfig, DEFAULT_PORT};
pub use web_client::MOBILE_WEB_CLIENT_HTML;
