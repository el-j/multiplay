//! Pure-Rust embedded WebSocket remote daemon and QR pairing adapter.

#![forbid(unsafe_code)]

pub mod pairing;
pub mod protocol;
pub mod server;
pub mod web_client;

pub use pairing::{PairingManager, QrPairingRenderer};
pub use protocol::{RemoteAction, RemoteEventBroadcast};
pub use server::RemoteDaemon;
pub use web_client::MOBILE_WEB_CLIENT_HTML;

