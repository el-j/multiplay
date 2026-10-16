//! Pure-Rust embedded WebSocket remote daemon and QR pairing adapter.

#![forbid(unsafe_code)]

pub mod pairing;
pub mod protocol;
pub mod server;

pub use pairing::{PairingManager, QrPairingRenderer};
pub use protocol::{RemoteAction, RemoteEventBroadcast};
pub use server::RemoteDaemon;
