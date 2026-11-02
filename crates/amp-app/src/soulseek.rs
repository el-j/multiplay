//! Soulseek in the terminal app (TASK-162): when "Use Soulseek" is on in the desktop app's
//! settings, the TUI connects to the same slskd (the user's own, or the one the desktop app
//! runs) with the API key from the shared vault. It never installs or starts slskd itself.

#![forbid(unsafe_code)]

use amp_adapter_soulseek::{vault_keys, SlskdClient, SoulseekProvider};
use amp_domain::config::{AppConfig, SoulseekMode};
use amp_ports::CredentialVaultPort;
use std::path::PathBuf;
use std::sync::Arc;

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// The provider, connected, when Soulseek is switched on; `None` otherwise.
#[must_use]
pub fn connect_from_settings() -> Option<Arc<SoulseekProvider>> {
    let home = home()?;
    let text = std::fs::read_to_string(home.join(".config/multiplay/config.toml")).ok()?;
    let config: AppConfig = toml::from_str(&text).ok()?;
    let s = config.soulseek;
    if !s.enabled {
        return None;
    }
    let (url, key_name) = match s.mode {
        SoulseekMode::External => (s.url, vault_keys::EXTERNAL_API_KEY),
        SoulseekMode::Managed => (
            format!("http://127.0.0.1:{}", s.managed_port),
            vault_keys::MANAGED_API_KEY,
        ),
    };
    let vault = amp_adapter_storage::KeyringStorageAdapter::with_vault_file(
        home.join(".multiplay/vault.bin"),
    )
    .ok()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    let key = rt.block_on(vault.get_secret(key_name)).ok().flatten()?;
    let provider = Arc::new(SoulseekProvider::new());
    provider.connect(SlskdClient::new(&url, key.expose_secret()).ok()?);
    Some(provider)
}
