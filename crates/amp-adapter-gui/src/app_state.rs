//! Application state container for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::dj_panel::DjConsoleState;
use crate::docking::{DockManager, WindowId};
use crate::settings_panel::GuiSettingsTab;
use amp_adapter_storage::{collect_audio_files, extract_track_from_file};
use amp_domain::source_badge::SourceKind;
use amp_domain::{AppConfig, AudioSource, Track};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

/// Core GUI application state container.
pub struct MultiplayGuiApp {
    pub audio_driver: AudioPlaybackDriver,
    pub playlist: Vec<Track>,
    pub selected_track_idx: usize,
    pub search_query: String,
    pub filter_source: Option<SourceKind>,
    pub dock_manager: DockManager,
    pub initialized_viewports: HashSet<WindowId>,
    pub was_dragging: HashSet<WindowId>,
    pub dj_state: DjConsoleState,
    pub settings_tab: GuiSettingsTab,
    pub config: AppConfig,
    pub volume: f32,
    pub eq_preamp: f32,
    pub eq_gains: [f32; 10],
    pub eq_enabled: bool,
}

impl MultiplayGuiApp {
    /// Initializes GUI state, audio driver, layout persistence, and standard music directories.
    pub fn new() -> Result<Self, String> {
        let audio_driver = AudioPlaybackDriver::new()?;
        let mut app = Self {
            audio_driver,
            playlist: Vec::new(),
            selected_track_idx: 0,
            search_query: String::new(),
            filter_source: None,
            dock_manager: DockManager::default(),
            initialized_viewports: HashSet::new(),
            was_dragging: HashSet::new(),
            dj_state: DjConsoleState::default(),
            settings_tab: GuiSettingsTab::default(),
            config: AppConfig::default(),
            volume: 0.85,
            eq_preamp: 0.0,
            eq_gains: [0.0; 10],
            eq_enabled: true,
        };

        app.load_layout();
        app.scan_user_music_dir();
        Ok(app)
    }

    /// Ingests dropped or selected file/directory path into playlist.
    pub fn ingest_path(&mut self, path: &Path) {
        if path.is_dir() {
            let files = collect_audio_files(path);
            for f in files {
                if let Some(track) = extract_track_from_file(&f) {
                    self.playlist.push(track);
                }
            }
        } else if let Some(track) = extract_track_from_file(path) {
            self.playlist.push(track);
        }
    }

    /// Ingests a stream URL (e.g. YouTube, Spotify, HTTP stream).
    pub fn ingest_url(&mut self, url: &str) {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return;
        }

        let (source, title) = if trimmed.contains("youtube.com") || trimmed.contains("youtu.be") {
            (AudioSource::YouTube { video_id: trimmed.to_string() }, "YouTube Stream")
        } else if trimmed.contains("spotify.com") {
            (AudioSource::Spotify { track_uri: trimmed.to_string() }, "Spotify Track")
        } else {
            (AudioSource::DirectHttp { url: trimmed.to_string() }, "Direct HTTP Stream")
        };

        let meta = amp_domain::TrackMetadata::new(title, "Network Source");
        self.playlist.push(Track::new(source, meta));
    }

    /// Scans standard user ~/Music folder.
    pub fn scan_user_music_dir(&mut self) {
        if let Some(home) = std::env::var_os("HOME") {
            let music_dir = PathBuf::from(home).join("Music");
            if music_dir.exists() {
                self.ingest_path(&music_dir);
            }
        }
    }

    /// Fallback file picker action.
    pub fn open_file_dialog(&mut self) {
        self.scan_user_music_dir();
    }

    /// Fallback folder picker action.
    pub fn open_folder_dialog(&mut self) {
        self.scan_user_music_dir();
    }

    /// Checks if active track reached EOF and automatically triggers gapless advance.
    pub fn check_gapless_advancement(&mut self) {
        if self.audio_driver.track_finished.swap(false, Ordering::SeqCst)
            && !self.playlist.is_empty()
            && self.selected_track_idx + 1 < self.playlist.len()
        {
            self.selected_track_idx += 1;
            self.audio_driver.play(self.playlist[self.selected_track_idx].clone());
        }
    }

    /// Saves modular window layout to configuration directory.
    pub fn save_layout(&self) {
        if let Some(home) = std::env::var_os("HOME") {
            let config_dir = PathBuf::from(home).join(".config/multiplay");
            let _ = std::fs::create_dir_all(&config_dir);
            let layout_file = config_dir.join("window_layout.json");
            if let Ok(json) = serde_json::to_string_pretty(&self.dock_manager) {
                let _ = std::fs::write(layout_file, json);
            }
        }
    }

    /// Loads modular window layout from configuration directory.
    pub fn load_layout(&mut self) {
        if let Some(home) = std::env::var_os("HOME") {
            let layout_file = PathBuf::from(home).join(".config/multiplay/window_layout.json");
            if let Ok(json) = std::fs::read_to_string(layout_file) {
                if let Ok(dm) = serde_json::from_str::<DockManager>(&json) {
                    self.dock_manager = dm;
                }
            }
        }
    }

    /// Synchronizes positions of all transitively docked satellite windows when parent window moves.
    pub fn sync_child_viewport_positions(&self, ctx: &eframe::egui::Context) {
        for (&child, &(parent, _)) in &self.dock_manager.dock_parent {
            if parent == WindowId::Main && self.dock_manager.is_visible(child) {
                let rect = self.dock_manager.get_rect(child);
                let viewport_id = match child {
                    WindowId::Equalizer => eframe::egui::ViewportId::from_hash_of("win_eq"),
                    WindowId::Playlist => eframe::egui::ViewportId::from_hash_of("win_playlist"),
                    WindowId::Settings => eframe::egui::ViewportId::from_hash_of("win_settings"),
                    WindowId::DjConsole => eframe::egui::ViewportId::from_hash_of("win_dj"),
                    WindowId::Main => continue,
                };
                ctx.send_viewport_cmd_to(
                    viewport_id,
                    eframe::egui::ViewportCommand::OuterPosition(eframe::egui::Pos2::new(rect.x, rect.y)),
                );
            }
        }
    }
}
