//! Pure-Rust embedded WebSocket remote daemon and QR pairing adapter.
//!
//! The daemon is off unless a front end starts it, binds loopback unless LAN access is
//! requested, and only accepts clients that paired with a host-issued token.

#![forbid(unsafe_code)]

pub mod auth;
pub mod handshake;
pub mod http;
pub mod pairing;
pub mod protocol;
pub mod server;
pub mod web_client;

pub use pairing::QrPairingRenderer;
pub use protocol::{parse_client_message, RemoteAction, RemoteEventBroadcast, RemoteTrackInfo};
pub use server::{RemoteDaemon, RemoteDaemonConfig, DEFAULT_PORT};
pub use web_client::MOBILE_WEB_CLIENT_HTML;
