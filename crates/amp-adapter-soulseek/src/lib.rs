//! "Use Soulseek" (TASK-162): multiplay does not implement the Soulseek protocol. It talks to
//! slskd, a separate Soulseek client with a REST API, either one the user already runs or one
//! multiplay installs and runs on request (`install`, `daemon`).

#![forbid(unsafe_code)]

pub mod daemon;
pub mod install;
pub mod provider;
pub mod slskd;

pub use provider::{metadata_from_path, SoulseekProvider, DOWNLOAD_TIMEOUT};

/// Vault entry names (the encrypted vault in multiplay's data folder).
pub mod vault_keys {
    /// The Soulseek account password for the managed slskd.
    pub const PASSWORD: &str = "soulseek_password";
    /// The API key multiplay generated for the slskd it runs.
    pub const MANAGED_API_KEY: &str = "slskd_managed_api_key";
    /// The API key of the user's own slskd.
    pub const EXTERNAL_API_KEY: &str = "slskd_external_api_key";
}
pub use slskd::{SlskdClient, SlskdError};
