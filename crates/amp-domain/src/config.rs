//! Unified hierarchical configuration schema for multiplay.
//! Pure domain data structures with zero external C dependencies.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level application configuration schema.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub library: LibraryConfig,
    pub audio: AudioHardwareConfig,
    pub streaming: StreamingProvidersConfig,
    pub p2p: P2PNetworkConfig,
    pub ui: UiAndThemeConfig,
    pub plugins: HashMap<String, String>,
    /// Phone/browser remote control. Missing in older config files, so it defaults (off).
    #[serde(default)]
    pub remote: RemoteControlConfig,
}

/// Remote-control daemon settings. Off and loopback-only unless the user opts in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteControlConfig {
    /// Start the remote-control daemon.
    pub enabled: bool,
    /// Listen on all network interfaces instead of loopback only.
    pub allow_lan: bool,
    /// TCP port to listen on.
    pub port: u16,
}

impl Default for RemoteControlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_lan: false,
            port: 9002,
        }
    }
}

/// Music library indexing and filesystem watching configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub music_directories: Vec<String>,
    pub watch_filesystem: bool,
    pub rescan_interval_secs: u64,
    pub supported_extensions: Vec<String>,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            music_directories: vec!["~/Music".into()],
            watch_filesystem: true,
            rescan_interval_secs: 3600,
            supported_extensions: crate::formats::playable_extensions()
                .map(String::from)
                .collect(),
        }
    }
}

/// Audio output hardware driver and buffer configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioHardwareConfig {
    pub output_device: Option<String>,
    pub sample_rate: u32,
    pub buffer_frames: u32,
    pub precache_ram_mb: usize,
    pub crossfade_duration_ms: u32,
    pub loudness_normalization: bool,
    pub target_lufs: i32, // Stored as integer * 10 or dB (e.g. -14)
    pub headphone_crossfeed: String,
    /// Output device for the DJ headphone (cue) mix, `None` for none. Missing in old files.
    #[serde(default)]
    pub dj_headphones_device: Option<String>,
}

impl Default for AudioHardwareConfig {
    fn default() -> Self {
        Self {
            output_device: None,
            sample_rate: 48000,
            buffer_frames: 1024,
            precache_ram_mb: 64,
            crossfade_duration_ms: 0,
            loudness_normalization: true,
            target_lufs: -14,
            headphone_crossfeed: "Off".to_string(),
            dj_headphones_device: None,
        }
    }
}

/// Online streaming service integrations configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamingProvidersConfig {
    pub enable_youtube: bool,
    pub enable_spotify: bool,
    pub enable_soundcloud: bool,
    pub enable_bandcamp: bool,
    pub download_cache_dir: String,
}

impl Default for StreamingProvidersConfig {
    fn default() -> Self {
        Self {
            enable_youtube: true,
            enable_spotify: false,
            enable_soundcloud: true,
            enable_bandcamp: true,
            download_cache_dir: "~/.multiplay/downloads".into(),
        }
    }
}

/// Decentralized P2P mesh and Soulseek network configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct P2PNetworkConfig {
    pub enable_p2p_mesh: bool,
    pub enable_soulseek: bool,
    pub mesh_listen_port: u16,
    pub soulseek_listen_port: u16,
    pub max_upload_slots: u16,
    pub shared_directories: Vec<String>,
}

impl Default for P2PNetworkConfig {
    fn default() -> Self {
        Self {
            enable_p2p_mesh: true,
            enable_soulseek: false,
            mesh_listen_port: 4040,
            soulseek_listen_port: 2234,
            max_upload_slots: 4,
            shared_directories: Vec::new(),
        }
    }
}

/// UI layout, skin theme, and visualizer configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAndThemeConfig {
    pub active_skin_path: Option<String>,
    pub visualizer_fps: u32,
    pub magnetic_snap_distance: u16,
    pub compact_mode: bool,
    pub show_equalizer: bool,
    pub show_playlist: bool,
    pub show_dj_console: bool,
    /// The first-run setup was finished or skipped (TASK-171). Missing in older configs, so
    /// people upgrading see it once.
    #[serde(default)]
    pub first_run_done: bool,
}

impl Default for UiAndThemeConfig {
    fn default() -> Self {
        Self {
            active_skin_path: None,
            visualizer_fps: 60,
            magnetic_snap_distance: 10,
            compact_mode: false,
            show_equalizer: true,
            show_playlist: true,
            show_dj_console: false,
            first_run_done: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_schema_serialization_roundtrip() {
        let mut config = AppConfig::default();
        config.audio.sample_rate = 96000;
        config
            .library
            .music_directories
            .push("/Volumes/HiResAudio".into());
        config.plugins.insert("dsp_reverb".into(), "enabled".into());

        let json = serde_json::to_string_pretty(&config).expect("Serialization failed");
        let deserialized: AppConfig = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(config, deserialized);
        assert_eq!(deserialized.audio.sample_rate, 96000);
        assert_eq!(
            deserialized.plugins.get("dsp_reverb"),
            Some(&"enabled".to_string())
        );
    }

    #[test]
    fn files_from_the_dj_beta_with_an_experimental_section_still_load() {
        let mut old = serde_json::to_value(AppConfig::default()).unwrap();
        old.as_object_mut().unwrap().insert(
            "experimental".into(),
            serde_json::json!({ "dj_mode": true }),
        );
        assert!(serde_json::from_value::<AppConfig>(old).is_ok());
    }

    #[test]
    fn remote_control_is_off_and_loopback_by_default() {
        let config = AppConfig::default();
        assert!(!config.remote.enabled);
        assert!(!config.remote.allow_lan);

        // Config files written before the `remote` section existed still load.
        let mut json: serde_json::Value = serde_json::to_value(&config).unwrap();
        json.as_object_mut().unwrap().remove("remote");
        let legacy: AppConfig = serde_json::from_value(json).expect("legacy config must load");
        assert_eq!(legacy.remote, RemoteControlConfig::default());
    }
}
