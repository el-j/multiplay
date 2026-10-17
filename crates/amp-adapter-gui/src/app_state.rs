//! Application state container for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::dj_panel::DjConsoleState;
use crate::docking::{DockManager, WindowId};
use crate::settings_panel::GuiSettingsTab;
use amp_adapter_storage::{
    collect_audio_files, export_m3u8, extract_track_from_file, import_m3u8, SessionPlaylistSnapshot,
};
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
    pub theme: amp_core_engine::WinampTheme,
    pub skin_bitmaps: std::collections::HashMap<String, Vec<u8>>,
    pub skin_cache: crate::skin_texture::SkinTextureCache,
    pub visualizer_mode: crate::visualizer_widget::VisualizerMode,
    pub remote: crate::remote_manager::RemoteGuiManager,
}

impl MultiplayGuiApp {
    /// Initializes GUI state, audio driver, layout persistence, and standard music directories.
    pub fn new() -> Result<Self, String> {
        let audio_driver = AudioPlaybackDriver::new()?;
        let mut remote = crate::remote_manager::RemoteGuiManager::new();
        remote.start_daemon();

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
            theme: amp_core_engine::WinampTheme::default(),
            skin_bitmaps: std::collections::HashMap::new(),
            skin_cache: crate::skin_texture::SkinTextureCache::default(),
            visualizer_mode: crate::visualizer_widget::VisualizerMode::Spectrum,
            remote,
        };

        app.load_layout();
        if !app.load_session() {
            app.scan_user_music_dir();
        }
        Ok(app)
    }

    /// Loads last active session playlist and EQ parameters from disk.
    pub fn load_session(&mut self) -> bool {
        let path = SessionPlaylistSnapshot::default_session_path();
        if let Ok(snap) = SessionPlaylistSnapshot::load_from_file(&path) {
            self.playlist = snap.tracks;
            self.selected_track_idx = if self.playlist.is_empty() {
                0
            } else {
                snap.selected_idx.min(self.playlist.len() - 1)
            };
            self.volume = snap.volume;
            self.eq_preamp = snap.eq_preamp;
            self.eq_gains = snap.eq_gains;
            self.eq_enabled = snap.eq_enabled;

            self.audio_driver.set_volume(self.volume);
            self.audio_driver.set_eq_preamp(self.eq_preamp);
            for (i, &g) in self.eq_gains.iter().enumerate() {
                self.audio_driver.set_eq_band(i, g);
            }
            self.audio_driver
                .eq_enabled
                .store(self.eq_enabled, Ordering::SeqCst);
            !self.playlist.is_empty()
        } else {
            false
        }
    }

    /// Persists current playlist and playback configuration to session snapshot.
    pub fn save_session(&self) {
        let snap = SessionPlaylistSnapshot {
            tracks: self.playlist.clone(),
            selected_idx: self.selected_track_idx,
            volume: self.volume,
            eq_preamp: self.eq_preamp,
            eq_gains: self.eq_gains,
            eq_enabled: self.eq_enabled,
        };
        let path = SessionPlaylistSnapshot::default_session_path();
        let _ = snap.save_to_file(&path);
    }

    /// Exports current playlist to an M3U8 playlist file.
    pub fn export_playlist_to_m3u8(&self, dest_path: &Path) -> Result<(), String> {
        let content = export_m3u8(&self.playlist);
        std::fs::write(dest_path, content).map_err(|e| format!("Failed to export M3U8: {e}"))
    }

    /// Imports tracks from an M3U8 playlist file.
    pub fn import_playlist_from_m3u8(&mut self, src_path: &Path) -> Result<usize, String> {
        let content = std::fs::read_to_string(src_path)
            .map_err(|e| format!("Failed to read M3U8 file: {e}"))?;
        let tracks = import_m3u8(&content);
        let count = tracks.len();
        self.playlist.extend(tracks);
        self.save_session();
        Ok(count)
    }

    /// Loads a .wsz Winamp classic skin from a file path.
    pub fn load_skin_from_path(&mut self, path: &Path) -> Result<(), String> {
        let skin = amp_core_engine::SkinLoader::load_skin_from_wsz_file(path)?;
        self.theme = skin.theme;
        self.skin_bitmaps = skin.bitmaps;
        self.skin_cache.clear();
        Ok(())
    }

    /// Opens native OS file dialog to pick a .wsz Winamp skin file.
    pub fn open_skin_dialog(&mut self) {
        let files = crate::file_dialog::pick_audio_files();
        for file in files {
            if let Some(ext) = file.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("wsz") {
                    let _ = self.load_skin_from_path(&file);
                    break;
                }
            }
        }
    }

    /// Ingests dropped or selected file/directory path into playlist.
    pub fn ingest_path(&mut self, path: &Path) {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("wsz") {
                let _ = self.load_skin_from_path(path);
                return;
            }
            if ext.eq_ignore_ascii_case("m3u") || ext.eq_ignore_ascii_case("m3u8") {
                let _ = self.import_playlist_from_m3u8(path);
                return;
            }
        }

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
        self.save_session();
    }

    /// Ingests a stream URL (e.g. YouTube, Spotify, HTTP stream).
    pub fn ingest_url(&mut self, url: &str) {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return;
        }

        let s = trimmed.to_string();
        let (source, title) = if s.contains("youtube.com") || s.contains("youtu.be") {
            (AudioSource::YouTube { video_id: s }, "YouTube Stream")
        } else if s.contains("spotify.com") {
            (AudioSource::Spotify { track_uri: s }, "Spotify Track")
        } else {
            (AudioSource::DirectHttp { url: s }, "Direct HTTP Stream")
        };

        let meta = amp_domain::TrackMetadata::new(title, "Network Source");
        self.playlist.push(Track::new(source, meta));
        self.save_session();
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

    /// Opens native OS file dialog to pick audio files or skins.
    pub fn open_file_dialog(&mut self) {
        let files = crate::file_dialog::pick_audio_files();
        if files.is_empty() {
            self.scan_user_music_dir();
        } else {
            for file in files {
                self.ingest_path(&file);
            }
        }
    }

    /// Opens native OS folder dialog to pick a music folder.
    pub fn open_folder_dialog(&mut self) {
        if let Some(folder) = crate::file_dialog::pick_folder() {
            self.ingest_path(&folder);
        } else {
            self.scan_user_music_dir();
        }
    }

    /// Checks if active track reached EOF or automatically transitioned via lookahead feeder.
    pub fn check_gapless_advancement(&mut self) {
        if self
            .audio_driver
            .track_advanced_signal
            .swap(false, Ordering::SeqCst)
        {
            if !self.playlist.is_empty() && self.selected_track_idx + 1 < self.playlist.len() {
                self.selected_track_idx += 1;
                let upcoming = self.playlist.get(self.selected_track_idx + 1).cloned();
                self.audio_driver.set_next_track(upcoming);
            }
            return;
        }

        let finished = self
            .audio_driver
            .track_finished
            .swap(false, Ordering::SeqCst);
        if finished
            && !self.playlist.is_empty()
            && self.selected_track_idx + 1 < self.playlist.len()
        {
            self.selected_track_idx += 1;
            let next = self.playlist[self.selected_track_idx].clone();
            let upcoming = self.playlist.get(self.selected_track_idx + 1).cloned();
            self.audio_driver.set_next_track(upcoming);
            self.audio_driver.play(next);
        }
    }

    /// Saves modular window layout to configuration directory.
    pub fn save_layout(&self) {
        crate::layout_manager::save_layout(&self.dock_manager);
    }

    /// Loads modular window layout from configuration directory.
    pub fn load_layout(&mut self) {
        if let Some(dm) = crate::layout_manager::load_layout() {
            self.dock_manager = dm;
        }
    }

    /// Synchronizes positions of all transitively docked satellite windows when parent window moves.
    pub fn sync_child_viewport_positions(&self, ctx: &eframe::egui::Context) {
        crate::layout_manager::sync_child_viewport_positions(&self.dock_manager, ctx);
    }
}
