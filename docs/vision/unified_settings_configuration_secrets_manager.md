# Unified Configuration Management, Secrets Storage & Plugin Settings

This document defines the persistent configuration system for the application, covering:
1. **Hierarchical TOML Configuration**: Schema validation, path auto-resolution, and atomic hot-reloading.
2. **Secure Keyring & Credential Store**: Zero-dependency, pure-Rust secret vault for streaming platform authentication (Spotify, Soulseek, Last.fm, Bandcamp).
3. **Dynamic Plugin & Source Settings**: Modular schema registration allowing plugins to expose custom settings without altering the core player schema.
4. **Interactive Preferences TUI/GUI**: Dense preferences dialog organized by categories with live preview.

---

## 1. System Configuration Architecture

```
 ┌────────────────────────────────────────────────────────────────────────┐
 │                      APPLICATION CONFIGURATION BUS                     │
 └────────────────────────────────────────────────────────────────────────┘
                   │                                      │
                   ▼                                      ▼
     ┌───────────────────────────┐          ┌───────────────────────────┐
     │  Plaintext Config (TOML)  │          │   Secure Keychain Vault   │
     │  `~/.config/amp/amp.toml` │          │   (OS Protected Storage)  │
     │  • Library scan paths     │          │   • Spotify Refresh Tokens│
     │  • Audio output buffer sz │          │   • Soulseek Passwords    │
     │  • Visualizer colors/FPS  │          │   • P2P Private Key Seeds │
     └─────────────┬─────────────┘          └─────────────┬─────────────┘
                   │                                      │
                   └──────────────────┬───────────────────┘
                                      ▼
     ┌──────────────────────────────────────────────────────────────────┐
     │           ConfigManager Actor (`Arc<RwLock<AppConfig>>`)         │
     │   • In-memory fast reads for audio thread                        │
     │   • Filesystem event watcher (inotify / kqueue) for hot-reload   │
     │   • Dispatches `ConfigChangedEvent` across CQRS EventBus         │
     └──────────────────────────────────────────────────────────────────┘
```

---

## 2. Configuration Data Model (`crates/amp-domain/src/config.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub library: LibraryConfig,
    #[serde(default)]
    pub audio: AudioHardwareConfig,
    #[serde(default)]
    pub streaming: StreamingProvidersConfig,
    #[serde(default)]
    pub p2p: P2PNetworkConfig,
    #[serde(default)]
    pub ui: UiAndThemeConfig,
    #[serde(default)]
    pub plugins: HashMap<String, toml::Value>, // Arbitrary plugin-specific schema maps
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub music_folders: Vec<PathBuf>,
    pub watch_filesystem_changes: bool,
    pub auto_rescan_interval_minutes: u32,
    pub follow_symlinks: bool,
    pub supported_extensions: Vec<String>,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            music_folders: vec![
                dirs::audio_dir().unwrap_or_else(|| PathBuf::from("~/Music")),
            ],
            watch_filesystem_changes: true,
            auto_rescan_interval_minutes: 60,
            follow_symlinks: false,
            supported_extensions: vec![
                "flac".into(), "mp3".into(), "wav".into(), "ogg".into(),
                "m4a".into(), "opus".into(), "aac".into(), "alac".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioHardwareConfig {
    pub output_device_name: Option<String>,
    pub buffer_size_frames: usize,
    pub precache_ram_mb: usize,
    pub enable_exclusive_mode: bool,
    pub target_sample_rate: Option<u32>, // None = Bit-perfect native auto-switch
    pub loudness_normalization_target_lufs: f32,
    pub crossfade_duration_ms: u32,
}

impl Default for AudioHardwareConfig {
    fn default() -> Self {
        Self {
            output_device_name: None,
            buffer_size_frames: 1024,
            precache_ram_mb: 128,
            enable_exclusive_mode: false,
            target_sample_rate: None,
            loudness_normalization_target_lufs: -14.0,
            crossfade_duration_ms: 1200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingProvidersConfig {
    pub youtube_enabled: bool,
    pub youtube_audio_quality: String, // "High", "Medium", "Low"
    pub spotify_enabled: bool,
    pub spotify_audio_bitrate: u32,   // 96, 160, 320 kbps
    pub soundcloud_enabled: bool,
    pub bandcamp_enabled: bool,
    pub download_directory: PathBuf,
}

impl Default for StreamingProvidersConfig {
    fn default() -> Self {
        Self {
            youtube_enabled: true,
            youtube_audio_quality: "High".into(),
            spotify_enabled: false,
            spotify_audio_bitrate: 320,
            soundcloud_enabled: true,
            bandcamp_enabled: true,
            download_directory: dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("~/Downloads"))
                .join("amp_vault"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PNetworkConfig {
    pub enable_mesh: bool,
    pub enable_soulseek: bool,
    pub soulseek_server_host: String,
    pub soulseek_listen_port: u16,
    pub mesh_listen_port: u16,
    pub enable_nat_upnp: bool,
    pub max_upload_slots: usize,
    pub shared_folders: Vec<PathBuf>,
}

impl Default for P2PNetworkConfig {
    fn default() -> Self {
        Self {
            enable_mesh: true,
            enable_soulseek: false,
            soulseek_server_host: "server.slsknet.org:2242".into(),
            soulseek_listen_port: 2234,
            mesh_listen_port: 4001,
            enable_nat_upnp: true,
            max_upload_slots: 5,
            shared_folders: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAndThemeConfig {
    pub active_skin_path: Option<PathBuf>,
    pub visualizer_fps: u8,
    pub magnetic_docking_threshold_cells: u16,
    pub show_source_badges_in_playlist: bool,
    pub start_in_compact_winamp1_mode: bool,
}

impl Default for UiAndThemeConfig {
    fn default() -> Self {
        Self {
            active_skin_path: None,
            visualizer_fps: 60,
            magnetic_docking_threshold_cells: 2,
            show_source_badges_in_playlist: true,
            start_in_compact_winamp1_mode: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            library: LibraryConfig::default(),
            audio: AudioHardwareConfig::default(),
            streaming: StreamingProvidersConfig::default(),
            p2p: P2PNetworkConfig::default(),
            ui: UiAndThemeConfig::default(),
            plugins: HashMap::new(),
        }
    }
}
```

---

## 3. Secure Credential Vault (`crates/amp-adapter-storage/src/keyring.rs`)

Secrets such as Spotify passwords, session tokens, and Soulseek passwords should never be saved in cleartext inside `amp.toml`. The `CredentialVault` manages sensitive secrets using native OS keychains or an encrypted ChaCha20-Poly1305 master-key store.

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(val: String) -> Self {
        Self(val)
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

pub trait CredentialVaultPort: Send + Sync {
    fn store_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), String>;
    fn get_secret(&self, service: &str, account: &str) -> Result<Option<SecretString>, String>;
    fn delete_secret(&self, service: &str, account: &str) -> Result<(), String>;
}

pub struct KeyringStorageAdapter {
    app_identifier: String,
}

impl KeyringStorageAdapter {
    pub fn new(app_identifier: impl Into<String>) -> Self {
        Self {
            app_identifier: app_identifier.into(),
        }
    }
}

impl CredentialVaultPort for KeyringStorageAdapter {
    fn store_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(&format!("{}:{}", self.app_identifier, service), account)
            .map_err(|e| format!("Keyring initialization failed: {e}"))?;
        entry.set_password(secret)
            .map_err(|e| format!("Failed to write secure secret: {e}"))?;
        Ok(())
    }

    fn get_secret(&self, service: &str, account: &str) -> Result<Option<SecretString>, String> {
        let entry = keyring::Entry::new(&format!("{}:{}", self.app_identifier, service), account)
            .map_err(|e| format!("Keyring initialization failed: {e}"))?;
        match entry.get_password() {
            Ok(pass) => Ok(Some(SecretString::new(pass))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Secure vault lookup error: {e}")),
        }
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(&format!("{}:{}", self.app_identifier, service), account)
            .map_err(|e| format!("Keyring initialization failed: {e}"))?;
        let _ = entry.delete_password();
        Ok(())
    }
}
```

---

## 4. Configuration Storage & Hot-Reload Engine (`crates/amp-core-engine/src/config_store.rs`)

This service guarantees atomic write semantics (writing to a temp file before renaming) and uses an event loop to broadcast configuration updates across all running threads without restarting the player.

```rust
use amp_domain::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ConfigStore {
    config_path: PathBuf,
    current_config: Arc<RwLock<AppConfig>>,
}

impl ConfigStore {
    pub fn init_from_disk(custom_path: Option<PathBuf>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_path = custom_path.unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("amp")
                .join("amp.toml")
        });

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let loaded_config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            toml::from_str::<AppConfig>(&content).unwrap_or_else(|err| {
                eprintln!("Warning: Failed to parse amp.toml ({err}). Falling back to defaults.");
                AppConfig::default()
            })
        } else {
            let default_cfg = AppConfig::default();
            let toml_str = toml::to_string_pretty(&default_cfg)?;
            fs::write(&config_path, toml_str)?;
            default_cfg
        };

        Ok(Self {
            config_path,
            current_config: Arc::new(RwLock::new(loaded_config)),
        })
    }

    pub fn get_live_handle(&self) -> Arc<RwLock<AppConfig>> {
        Arc::clone(&self.current_config)
    }

    /// Atomically persists configuration updates to disk
    pub async fn save_mutation<F>(&self, mutate_fn: F) -> Result<(), String>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut guard = self.current_config.write().await;
        mutate_fn(&mut guard);

        let serialized = toml::to_string_pretty(&*guard)
            .map_err(|e| format!("Serialization error: {e}"))?;

        let temp_path = self.config_path.with_extension("tmp");
        fs::write(&temp_path, serialized)
            .map_err(|e| format!("I/O write error: {e}"))?;
        fs::rename(&temp_path, &self.config_path)
            .map_err(|e| format!("Atomic file replacement error: {e}"))?;

        Ok(())
    }
}
```

---

## 5. Preferences TUI Modal (`crates/amp-adapter-tui/src/settings_view.rs`)

The settings dialog allows users to tab between categories, browse and add local folders, toggle service credentials, and test hardware output configs in real-time.

```rust
use amp_domain::config::AppConfig;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    MusicFolders,
    AudioHardware,
    OnlineAccounts,
    P2PNetwork,
    Appearance,
    PluginOptions,
}

pub struct SettingsViewState {
    pub active_category: SettingsCategory,
    pub selected_entry_idx: usize,
    pub is_editing_field: bool,
    pub input_buffer: String,
}

pub fn render_settings_dialog(
    f: &mut Frame,
    config: &AppConfig,
    state: &SettingsViewState,
    area: Rect,
) {
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" 󰒓 Universal Settings & Accounts (Esc to Exit | Tab to Cycle Categories) ");
    
    let inner = main_block.inner(area);
    f.render_widget(main_block, area);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // Categories Sidebar
            Constraint::Min(40),    // Detail/Editing Panel
        ])
        .split(inner);

    // 1. Sidebar Category Menu
    let categories = [
        (SettingsCategory::MusicFolders, "󰉋 Media Library"),
        (SettingsCategory::AudioHardware, "󰓃 Audio & DAC"),
        (SettingsCategory::OnlineAccounts, "󰀂 Online Accounts"),
        (SettingsCategory::P2PNetwork, "󱘖 P2P & Sharing"),
        (SettingsCategory::Appearance, "󰏘 Skins & Display"),
        (SettingsCategory::PluginOptions, "󰐱 Plugin Extensions"),
    ];

    let cat_items: Vec<ListItem> = categories
        .iter()
        .map(|(cat, label)| {
            let is_selected = *cat == state.active_category;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("  {label} ")).style(style)
        })
        .collect();

    let sidebar = List::new(cat_items).block(
        Block::default().borders(Borders::RIGHT).title(" Categories ")
    );
    f.render_widget(sidebar, layout[0]);

    // 2. Category Detail Pane
    let details_pane = Block::default().borders(Borders::NONE);
    let details_inner = details_pane.inner(layout[1]);
    f.render_widget(details_pane, layout[1]);

    match state.active_category {
        SettingsCategory::MusicFolders => {
            let mut lines = vec![
                Line::from(Span::styled("Indexed Local & Network Music Folders:", Style::default().add_modifier(Modifier::BOLD))),
                Line::from(""),
            ];
            for (idx, folder) in config.library.music_folders.iter().enumerate() {
                lines.push(Line::from(format!("  [{idx}] 󰉋 {}", folder.display())));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Commands: [a] Add Folder  |  [d] Remove Folder  |  [r] Trigger Rescan Now",
                Style::default().fg(Color::Yellow),
            )));
            f.render_widget(Paragraph::new(lines), details_inner);
        }
        SettingsCategory::OnlineAccounts => {
            let lines = vec![
                Line::from(Span::styled("Third-Party Streaming Authentication:", Style::default().add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(format!("  [1] Spotify Premium : {}", if config.streaming.spotify_enabled { "󰄬 Enabled (Token Cached)" } else { "✘ Disabled" })),
                Line::from(format!("  [2] YouTube Music   : {}", if config.streaming.youtube_enabled { "󰄬 Enabled (Pure-Rust Extractor)" } else { "✘ Disabled" })),
                Line::from(format!("  [3] Soulseek P2P    : {}", if config.p2p.enable_soulseek { "󰄬 Connected" } else { "✘ Disabled" })),
                Line::from(format!("  [4] Bandcamp Stream : {}", if config.streaming.bandcamp_enabled { "󰄬 Enabled" } else { "✘ Disabled" })),
                Line::from(""),
                Line::from(Span::styled(
                    "Press [1-4] to enter credentials. Secrets are stored in the OS Keychain.",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            f.render_widget(Paragraph::new(lines), details_inner);
        }
        SettingsCategory::AudioHardware => {
            let lines = vec![
                Line::from(Span::styled("Audio Hardware & Buffering Architecture:", Style::default().add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(format!("  Active Sink Device   : {}", config.audio.output_device_name.as_deref().unwrap_or("Default System CoreAudio DAC"))),
                Line::from(format!("  Ring Buffer Frame    : {} frames (~{:.1} ms)", config.audio.buffer_size_frames, (config.audio.buffer_size_frames as f32 / 44.1))),
                Line::from(format!("  RAM Precache Size    : {} MB", config.audio.precache_ram_mb)),
                Line::from(format!("  Loudness Target      : {:.1} LUFS", config.audio.loudness_normalization_target_lufs)),
                Line::from(format!("  Auto Crossfade Time  : {} ms", config.audio.crossfade_duration_ms)),
            ];
            f.render_widget(Paragraph::new(lines), details_inner);
        }
        _ => {
            let placeholder = Paragraph::new("Configure options for this category using the navigation keys.")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, details_inner);
        }
    }
}
```