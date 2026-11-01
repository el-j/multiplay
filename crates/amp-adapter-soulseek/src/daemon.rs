//! Running the installed slskd while multiplay runs: on 127.0.0.1 only, plain HTTP (it never
//! leaves this computer), a generated API key, no shared folders. Settings, including the
//! Soulseek password, go in its environment, never on its command line, where other users could
//! read them in the process list. Dropping [`ManagedSlskd`] stops it.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// A fresh random API key (64 hex characters; slskd accepts 16 to 255).
#[must_use]
pub fn generate_api_key() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// How to run slskd.
#[derive(Debug, Clone)]
pub struct ManagedConfig {
    pub exe: PathBuf,
    /// slskd's own data folder (database, logs).
    pub app_dir: PathBuf,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub api_key: String,
    /// Where finished downloads go; the incomplete ones sit next to it.
    pub downloads: PathBuf,
}

/// A slskd started by multiplay.
pub struct ManagedSlskd {
    child: Child,
    port: u16,
}

impl ManagedSlskd {
    /// Starts slskd.
    ///
    /// # Errors
    /// When it cannot be started.
    pub fn start(config: &ManagedConfig) -> Result<Self, String> {
        std::fs::create_dir_all(&config.app_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&config.downloads).map_err(|e| e.to_string())?;
        let log = std::fs::File::create(config.app_dir.join("multiplay-slskd.log"))
            .map_err(|e| e.to_string())?;
        let child = Command::new(&config.exe)
            .current_dir(&config.app_dir)
            .env("APP_DIR", &config.app_dir)
            .env("SLSKD_HTTP_IP_ADDRESS", "127.0.0.1")
            .env("SLSKD_HTTP_PORT", config.port.to_string())
            .env("SLSKD_NO_HTTPS", "true")
            .env("SLSKD_SLSK_USERNAME", &config.username)
            .env("SLSKD_SLSK_PASSWORD", &config.password)
            .env("SLSKD_API_KEY", &config.api_key)
            .env("SLSKD_DOWNLOADS_DIR", &config.downloads)
            .env("SLSKD_INCOMPLETE_DIR", config.downloads.join(".incomplete"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .map_err(|e| format!("slskd could not be started: {e}"))?;
        tracing::info!(pid = child.id(), port = config.port, "slskd started");
        Ok(Self {
            child,
            port: config.port,
        })
    }

    /// Its API address.
    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Its process id.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// True while it runs.
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for ManagedSlskd {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        tracing::info!("slskd stopped");
    }
}
