//! The private mesh (ADR-009): devices that share a mesh code find each other, search each
//! other's shared music, stream it, and pass jam-session operations. libp2p with a pre-shared
//! key (pnet), Noise and Yamux; mDNS on the local network.

#![forbid(unsafe_code)]

mod catalog;
mod code;
mod discovery;
mod node;
mod protocol;
mod provider;

pub use catalog::SharedFile;
pub use code::{BadMeshCode, MeshCode};
pub use libp2p::Multiaddr;
pub use node::{MeshConfig, MeshNode};
pub use protocol::SharedTrack;
pub use provider::MeshProvider;

/// Vault keys for the mesh.
pub mod vault_keys {
    /// The mesh code.
    pub const MESH_CODE: &str = "mesh_code";
    /// This device's identity (so its peer id stays the same).
    pub const IDENTITY: &str = "mesh_identity";
}

/// A new device identity to save in the vault and pass as [`MeshConfig::identity`].
#[must_use]
pub fn new_identity() -> Vec<u8> {
    libp2p::identity::Keypair::generate_ed25519()
        .to_protobuf_encoding()
        .unwrap_or_default()
}
