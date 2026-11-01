//! "Use Soulseek" in the desktop app (TASK-162): connects the shared [`SoulseekProvider`] to the
//! user's own slskd, or installs (on request) and runs one. Setup work runs on a background
//! thread; secrets live in the encrypted vault, never in `config.toml`.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use amp_adapter_soulseek::daemon::{generate_api_key, ManagedConfig, ManagedSlskd};
use amp_adapter_soulseek::install::SLSKD_VERSION;
use amp_adapter_soulseek::{SlskdClient, SoulseekProvider};
use amp_domain::config::SoulseekMode;
use amp_ports::CredentialVaultPort;
use eframe::egui::{self, Color32, RichText, Ui};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use amp_adapter_soulseek::vault_keys::{
    EXTERNAL_API_KEY, MANAGED_API_KEY, PASSWORD as PASSWORD_KEY,
};

/// How long a freshly started slskd may take to answer.
const STARTUP_WAIT: Duration = Duration::from_secs(60);

/// The one Soulseek provider: search, the stream resolver and the setup share it.
pub fn soulseek() -> Arc<SoulseekProvider> {
    static PROVIDER: OnceLock<Arc<SoulseekProvider>> = OnceLock::new();
    Arc::clone(PROVIDER.get_or_init(|| Arc::new(SoulseekProvider::new())))
}

enum SetupResult {
    Connected {
        client: SlskdClient,
        version: String,
    },
    Failed(String),
    Installed(PathBuf),
}

/// Soulseek state the Preferences tab shows and changes.
#[derive(Default)]
pub struct SoulseekUi {
    /// Typed into the password / API key fields; saved to the vault on "Save".
    pub password_input: String,
    pub api_key_input: String,
    pub(crate) status: String,
    pub(crate) connected: bool,
    managed: Option<ManagedSlskd>,
    job: Option<Receiver<SetupResult>>,
}

/// What the Soulseek controls ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulseekAction {
    Apply,
    Install,
}

fn block_on<F: std::future::Future>(f: F) -> Option<F::Output> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()
        .map(|rt| rt.block_on(f))
}

impl MultiplayGuiApp {
    fn vault(&self) -> Option<amp_adapter_storage::KeyringStorageAdapter> {
        amp_adapter_storage::KeyringStorageAdapter::with_vault_file(
            self.paths.data_dir.join("vault.bin"),
        )
        .ok()
    }

    /// Stores a secret in the encrypted vault.
    pub fn save_soulseek_secret(&self, key: &str, value: &str) {
        if let Some(vault) = self.vault() {
            let _ = block_on(vault.store_secret(key, value));
        }
    }

    /// Reads a secret from the vault.
    #[must_use]
    pub fn soulseek_secret(&self, key: &str) -> Option<String> {
        let vault = self.vault()?;
        block_on(vault.get_secret(key))?
            .ok()
            .flatten()
            .map(|s| s.expose_secret().to_string())
    }

    fn slskd_dir(&self) -> PathBuf {
        self.paths.data_dir.join("slskd")
    }

    fn slskd_exe(&self) -> PathBuf {
        let name = if cfg!(windows) { "slskd.exe" } else { "slskd" };
        self.slskd_dir().join(SLSKD_VERSION).join(name)
    }

    /// True once slskd has been installed by multiplay.
    #[must_use]
    pub fn slskd_installed(&self) -> bool {
        self.slskd_exe().is_file()
    }

    /// One line on what Soulseek is doing.
    #[must_use]
    pub fn soulseek_status(&self) -> &str {
        &self.soulseek_ui.status
    }

    /// True while connecting or installing.
    #[must_use]
    pub fn soulseek_busy(&self) -> bool {
        self.soulseek_ui.job.is_some()
    }

    fn start_job(&mut self, status: &str, job: impl FnOnce() -> SetupResult + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("multiplay-soulseek-setup".into())
            .spawn(move || {
                let _ = tx.send(job());
            });
        if spawned.is_ok() {
            self.soulseek_ui.job = Some(rx);
            self.soulseek_ui.status = status.to_string();
        }
    }

    /// Makes Soulseek match the settings: off, connected to the user's slskd, or running ours.
    pub fn apply_soulseek(&mut self) {
        let settings = self.config.soulseek.clone();
        soulseek().disconnect();
        self.soulseek_ui.connected = false;
        if !settings.enabled || settings.mode == SoulseekMode::External {
            // Ours only runs in managed mode.
            self.soulseek_ui.managed = None;
        }
        if !settings.enabled {
            self.soulseek_ui.status = "Off".into();
            return;
        }
        match settings.mode {
            SoulseekMode::External => {
                let key = self.soulseek_secret(EXTERNAL_API_KEY).unwrap_or_default();
                let url = settings.url.clone();
                self.start_job("Connecting to slskd…", move || {
                    connect(&url, &key, Duration::ZERO)
                });
            }
            SoulseekMode::Managed => {
                if !self.slskd_installed() {
                    self.soulseek_ui.status =
                        "slskd is not installed. Install it to use Soulseek.".into();
                    return;
                }
                let password = self.soulseek_secret(PASSWORD_KEY).unwrap_or_default();
                if settings.username.is_empty() || password.is_empty() {
                    self.soulseek_ui.status =
                        "Enter your Soulseek user name and password, then Save.".into();
                    return;
                }
                let api_key = self.soulseek_secret(MANAGED_API_KEY).unwrap_or_else(|| {
                    let key = generate_api_key();
                    self.save_soulseek_secret(MANAGED_API_KEY, &key);
                    key
                });
                if self
                    .soulseek_ui
                    .managed
                    .as_mut()
                    .is_none_or(|m| !m.is_running())
                {
                    let config = ManagedConfig {
                        exe: self.slskd_exe(),
                        app_dir: self.slskd_dir().join("data"),
                        port: settings.managed_port,
                        username: settings.username.clone(),
                        password,
                        api_key: api_key.clone(),
                        downloads: self.slskd_dir().join("downloads"),
                    };
                    match ManagedSlskd::start(&config) {
                        Ok(m) => self.soulseek_ui.managed = Some(m),
                        Err(e) => {
                            self.soulseek_ui.status = e;
                            return;
                        }
                    }
                }
                let url = format!("http://127.0.0.1:{}", settings.managed_port);
                self.start_job("Starting slskd…", move || {
                    connect(&url, &api_key, STARTUP_WAIT)
                });
            }
        }
    }

    /// Downloads and installs slskd in the background, then starts it.
    pub fn install_slskd(&mut self) {
        let dir = self.slskd_dir();
        self.start_job(
            "Downloading slskd…",
            move || match amp_adapter_soulseek::install::install(&dir) {
                Ok(exe) => SetupResult::Installed(exe),
                Err(e) => SetupResult::Failed(format!("slskd could not be installed: {e}")),
            },
        );
    }

    /// Picks up a finished setup step.
    pub fn poll_soulseek(&mut self) {
        let Some(job) = &self.soulseek_ui.job else {
            return;
        };
        let result = match job.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => SetupResult::Failed("Soulseek setup stopped".into()),
        };
        self.soulseek_ui.job = None;
        match result {
            SetupResult::Connected { client, version } => {
                self.soulseek_ui.status =
                    format!("Connected to slskd {version} at {}", client.base());
                soulseek().connect(client);
                self.soulseek_ui.connected = true;
            }
            SetupResult::Failed(e) => self.soulseek_ui.status = e,
            SetupResult::Installed(exe) => {
                tracing::info!("slskd installed at {}", exe.display());
                self.apply_soulseek();
            }
        }
    }

    /// Runs a button from the Soulseek controls, saving typed secrets first.
    pub fn soulseek_action(&mut self, action: SoulseekAction) {
        let (password, key) = (
            std::mem::take(&mut self.soulseek_ui.password_input),
            std::mem::take(&mut self.soulseek_ui.api_key_input),
        );
        if !password.is_empty() {
            self.save_soulseek_secret(PASSWORD_KEY, &password);
        }
        if !key.is_empty() {
            self.save_soulseek_secret(EXTERNAL_API_KEY, &key);
        }
        self.save_and_apply_config();
        match action {
            SoulseekAction::Apply => self.apply_soulseek(),
            SoulseekAction::Install => self.install_slskd(),
        }
    }
}

/// Asks slskd for its version until it answers (up to `wait`).
fn connect(url: &str, key: &str, wait: Duration) -> SetupResult {
    let client = match SlskdClient::new(url, key) {
        Ok(c) => c,
        Err(e) => return SetupResult::Failed(e.to_string()),
    };
    let started = std::time::Instant::now();
    loop {
        match block_on(client.version()) {
            Some(Ok(version)) => return SetupResult::Connected { client, version },
            Some(Err(amp_adapter_soulseek::SlskdError::Unauthorized)) => {
                return SetupResult::Failed("slskd refused the API key".into())
            }
            Some(Err(e)) if started.elapsed() >= wait => return SetupResult::Failed(e.to_string()),
            _ => std::thread::sleep(Duration::from_millis(500)),
        }
    }
}

/// The Soulseek section of the P2P tab.
pub fn show_settings(
    ui: &mut Ui,
    config: &mut amp_domain::AppConfig,
    state: &mut SoulseekUi,
    installed: bool,
    action: &mut Option<SoulseekAction>,
) {
    let s = &mut config.soulseek;
    ui.add_space(6.0);
    ui.label(
        RichText::new("Soulseek")
            .strong()
            .color(Color32::from_rgb(0, 230, 140)),
    );
    let was = s.enabled;
    ui.checkbox(
        &mut s.enabled,
        "Use Soulseek (search other people's shared music)",
    );
    if s.enabled != was {
        *action = Some(SoulseekAction::Apply);
    }
    if !s.enabled {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.radio_value(
            &mut s.mode,
            SoulseekMode::Managed,
            "Install and run slskd for me",
        );
        ui.radio_value(&mut s.mode, SoulseekMode::External, "I run slskd myself");
    });
    match s.mode {
        SoulseekMode::Managed => {
            ui.label(
                RichText::new(
                    "multiplay downloads slskd, an open-source Soulseek client, from its GitHub \
                     releases and runs it on this computer only, sharing nothing. You need a \
                     Soulseek account; a new name and password create one.",
                )
                .size(10.0),
            );
            ui.horizontal(|ui| {
                ui.label("User name");
                ui.text_edit_singleline(&mut s.username);
            });
            ui.horizontal(|ui| {
                ui.label("Password");
                ui.add(
                    egui::TextEdit::singleline(&mut state.password_input)
                        .password(true)
                        .hint_text("saved encrypted"),
                );
            });
            ui.horizontal(|ui| {
                if !installed && ui.button("Install slskd").clicked() {
                    *action = Some(SoulseekAction::Install);
                }
                if ui.button("Save and connect").clicked() {
                    *action = Some(SoulseekAction::Apply);
                }
            });
        }
        SoulseekMode::External => {
            ui.horizontal(|ui| {
                ui.label("slskd address");
                ui.text_edit_singleline(&mut s.url);
            });
            ui.horizontal(|ui| {
                ui.label("API key");
                ui.add(
                    egui::TextEdit::singleline(&mut state.api_key_input)
                        .password(true)
                        .hint_text("from slskd.yml"),
                );
            });
            if ui.button("Save and connect").clicked() {
                *action = Some(SoulseekAction::Apply);
            }
        }
    }
    if !state.status.is_empty() {
        ui.label(RichText::new(&state.status).size(10.0));
    }
}
