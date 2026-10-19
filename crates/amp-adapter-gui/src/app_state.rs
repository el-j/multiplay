//! Application state container for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::docking::{DockManager, WindowId};
use crate::settings_panel::GuiSettingsTab;
use amp_adapter_storage::{
    collect_audio_files, export_m3u8, extract_track_from_file, import_m3u8, SessionPlaylistSnapshot,
};
use amp_domain::source_badge::SourceKind;
use amp_domain::{AppConfig, AudioSource, Track};
use std::collections::HashSet;
use std::path::Path;
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
    /// Title-bar drag in progress (app-driven, live magnetic snapping).
    pub window_drag: Option<crate::chrome::drag::DragSession>,
    /// Edge or grip resize in progress (app-driven, keeps windows apart).
    pub window_resize: Option<crate::chrome::resize::ResizeSession>,
    /// Heights to restore when a shaded window is rolled back down.
    pub shade_restore: std::collections::HashMap<WindowId, f32>,
    /// Zoom factor the dock layout is expressed in (2.0 in double size).
    pub applied_zoom: f32,
    /// Frames left in which OS geometry reports are ignored after a zoom change.
    pub geometry_settle: u8,
    /// Zoom the saved layout asks for, applied on the first frame.
    pub startup_zoom: f32,
    /// Last layout written to disk.
    pub saved_layout: Option<crate::layout_manager::LayoutSnapshot>,
    /// DJ mode session (created the first time DJ mode is turned on).
    pub dj: Option<crate::dj_session::DjSession>,
    /// Library database: DJ analysis cache and hot cues (opened on first use).
    pub library: Option<std::sync::Arc<amp_adapter_storage::RedbStorageAdapter>>,
    /// DJ deck of each playlist track (TASK-210), saved with the session.
    pub deck_assign: std::collections::HashMap<amp_domain::TrackId, amp_domain::DeckAssignment>,
    /// The DJ headphone device and its sink (kept open while selected).
    pub dj_phones: Option<(String, amp_adapter_cpal::CpalAudioSink)>,
    /// Why the chosen DJ headphone device could not be used.
    pub dj_phones_error: Option<String>,
    /// DJ console state outside the engine (loop sizes, cue points, overlays).
    pub dj_console: crate::dj_controls::ConsoleState,
    /// The DJ first-run hint was dismissed (saved with the session).
    pub dj_hint_done: bool,
    /// Tracks already loaded onto a deck in this DJ session.
    pub dj_played: HashSet<amp_domain::TrackId>,
    /// Main window size to restore when DJ mode ends.
    pub normal_main_size: Option<eframe::egui::Vec2>,
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
    /// Injected filesystem locations (user home in production, temp dirs in tests).
    pub paths: crate::app_paths::AppPaths,
}

impl MultiplayGuiApp {
    /// Initializes GUI state on the default audio device with the user's home directories.
    pub fn new() -> Result<Self, String> {
        Self::with_parts(
            AudioPlaybackDriver::new()?,
            crate::app_paths::AppPaths::for_user(),
        )
    }

    /// Hardware-free instance whose state lives under `root` (tests and CI).
    pub fn headless(root: &Path) -> Result<Self, String> {
        Self::with_parts(
            AudioPlaybackDriver::new_headless()?,
            crate::app_paths::AppPaths::in_dir(root),
        )
    }

    /// Initializes GUI state from an audio driver and filesystem locations.
    pub fn with_parts(
        audio_driver: AudioPlaybackDriver,
        paths: crate::app_paths::AppPaths,
    ) -> Result<Self, String> {
        // The remote daemon stays off until the user enables it (audit SEC-01).
        let remote =
            crate::remote_manager::RemoteGuiManager::new(Some(paths.remote_sessions_file()));

        let mut app = Self {
            audio_driver,
            playlist: Vec::new(),
            selected_track_idx: 0,
            search_query: String::new(),
            filter_source: None,
            dock_manager: DockManager::default(),
            initialized_viewports: HashSet::new(),
            window_drag: None,
            window_resize: None,
            shade_restore: std::collections::HashMap::new(),
            applied_zoom: 1.0,
            geometry_settle: 0,
            startup_zoom: 1.0,
            saved_layout: None,
            dj: None,
            library: None,
            normal_main_size: None,
            deck_assign: std::collections::HashMap::new(),
            dj_played: HashSet::new(),
            dj_console: Default::default(),
            dj_phones: None,
            dj_phones_error: None,
            dj_hint_done: false,
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
            paths,
        };

        app.load_layout();
        if !app.load_session() {
            app.scan_user_music_dir();
        }
        Ok(app)
    }

    /// Loads last active session playlist and EQ parameters from disk.
    pub fn load_session(&mut self) -> bool {
        let path = self.paths.session_file();
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
            self.deck_assign = snap.deck_assignments;
            self.dj_hint_done = snap.dj_hint_done;

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
            deck_assignments: self.deck_assign.clone(),
            dj_hint_done: self.dj_hint_done,
        };
        let _ = snap.save_to_file(&self.paths.session_file());
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

    /// Scans the configured music folder (normally ~/Music).
    pub fn scan_user_music_dir(&mut self) {
        if let Some(music_dir) = self.paths.music_dir.clone() {
            if music_dir.exists() {
                self.ingest_path(&music_dir);
            }
        }
    }

    /// Opens native OS file dialog to pick audio files or skins.
    pub fn open_file_dialog(&mut self) {
        // A cancelled dialog returns nothing and changes nothing (audit UX-07).
        for file in crate::file_dialog::pick_audio_files() {
            self.ingest_path(&file);
        }
    }

    /// Opens native OS folder dialog to pick a music folder.
    pub fn open_folder_dialog(&mut self) {
        if let Some(folder) = crate::file_dialog::pick_folder() {
            self.ingest_path(&folder);
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

    /// Current layout as it would be saved.
    pub fn layout_snapshot(&self) -> crate::layout_manager::LayoutSnapshot {
        crate::layout_manager::LayoutSnapshot::capture(
            &self.dock_manager,
            &self.shade_restore,
            self.applied_zoom,
        )
    }

    /// Saves the window layout to the configuration directory.
    pub fn save_layout(&mut self) {
        let snap = self.layout_snapshot();
        if crate::layout_manager::save_layout(&snap, &self.paths.layout_file).is_ok() {
            self.saved_layout = Some(snap);
        }
    }

    /// Saves the layout when it changed and no drag, resize or zoom change is under way.
    pub fn save_layout_if_changed(&mut self) {
        let busy =
            self.window_drag.is_some() || self.window_resize.is_some() || self.geometry_settle > 0;
        if !busy && self.saved_layout.as_ref() != Some(&self.layout_snapshot()) {
            self.save_layout();
        }
    }

    /// Loads the window layout (at 1x); its zoom is applied on the first frame.
    pub fn load_layout(&mut self) {
        if let Some(snap) = crate::layout_manager::load_layout(&self.paths.layout_file) {
            snap.apply(&mut self.dock_manager, &mut self.shade_restore);
            self.startup_zoom = snap.zoom;
            self.saved_layout = Some(snap);
        }
    }
}
