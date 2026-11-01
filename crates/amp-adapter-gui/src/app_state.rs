//! Application state container for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

use crate::docking::{DockManager, WindowId};
use crate::engine_factory::{create_headless_engine, create_player_engine};
use crate::settings_panel::GuiSettingsTab;
use amp_adapter_storage::{export_m3u8, SessionPlaylistSnapshot};
use amp_core_engine::PlayerEngine;
use amp_domain::source_badge::SourceKind;
use amp_domain::{AppConfig, AudioSource, Track};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;

/// Core GUI application state container.
pub struct MultiplayGuiApp {
    pub audio_driver: PlayerEngine,
    pub playlist: Vec<Track>,
    pub selected_track_idx: usize,
    pub search_query: String,
    pub filter_source: Option<SourceKind>,
    pub search_query_text: String,
    pub search_url_text: String,
    /// Ctrl+L asked for the cursor in the search window's link field.
    pub focus_location_field: bool,
    /// The open file dialog, if any (it runs on its own thread).
    pub dialog_job: Option<crate::dialog_jobs::DialogJob>,
    /// The system's media keys and Now Playing panel, when the OS offers one.
    pub media_controls: Option<Box<dyn amp_ports::MediaControlsPort>>,
    /// The library browser's index; `None` until the stored library has loaded.
    pub library_index: Option<amp_core_engine::library_index::LibraryIndex>,
    /// Bumped whenever the index is rebuilt, so cached views know to refresh.
    pub library_generation: u64,
    pub library_job: Option<std::sync::mpsc::Receiver<Vec<Track>>>,
    /// Imported while the library was still loading.
    pub library_pending: Vec<Track>,
    /// The Library window's view state and covers.
    pub library_ui: crate::library_window::LibraryUi,
    /// The first-run setup, while it is open (shown in the Preferences overlay).
    pub onboarding: Option<crate::onboarding::Onboarding>,
    /// The open folder dialog belongs to the setup.
    pub dialog_for_onboarding: bool,
    /// A background import (first-run folders, "Scan ~/Music").
    pub import_job: Option<crate::import_jobs::ImportJob>,
    /// A running update check, and a newer release it found (TASK-183).
    pub update_job: Option<crate::update_notice::UpdateJob>,
    pub update_available: Option<amp_domain::about::ReleaseInfo>,
    /// Scrubber drag state (seek on release).
    pub scrub: crate::transport::Scrub,
    pub search_results: Vec<Track>,
    pub is_searching: bool,
    pub search_status: Option<String>,
    /// Audio left out of the last import because its format cannot be played yet.
    pub import_notice: Option<String>,
    /// Why the last track could not be played, until another one starts.
    pub playback_notice: Option<String>,
    pub(crate) playback_events:
        Option<tokio::sync::broadcast::Receiver<amp_core_engine::events::PlayerEvent>>,
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
    pub config_store: amp_core_engine::ConfigStore,
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
    /// Phone requests waiting for the host (Remote window).
    pub remote_requests: crate::remote_requests::RequestQueue,
    /// Who requested each playlist track that came from a phone (saved with the session).
    pub requested_by: std::collections::HashMap<amp_domain::TrackId, String>,
    /// Accepted requests whose phone still waits for "playing".
    pub request_ids: std::collections::HashMap<amp_domain::TrackId, amp_adapter_remote::RequestId>,
    /// Resolvers for request previews; `None` uses the real providers (tests inject fixtures).
    pub lookup_services: Option<crate::remote_lookup::LookupServices>,
    /// Lookup thread, started by the first request that needs it.
    pub remote_lookup: Option<crate::remote_lookup::RemoteLookup>,
    /// Track last reported as playing to phones.
    pub last_playing: Option<amp_domain::TrackId>,
    /// Injected filesystem locations (user home in production, temp dirs in tests).
    pub paths: crate::app_paths::AppPaths,
}

impl MultiplayGuiApp {
    /// Initializes GUI state on the default audio device with the user's home directories.
    pub fn new() -> Result<Self, String> {
        let mut app = Self::with_parts(
            create_player_engine()?,
            crate::app_paths::AppPaths::for_user(),
        )?;
        app.load_library();
        if !app.config.ui.first_run_done {
            app.start_onboarding();
        } else if app.playlist.is_empty() {
            app.scan_user_music_dir();
        }
        Ok(app)
    }

    /// Hardware-free instance whose state lives under `root` (tests and CI).
    pub fn headless(root: &Path) -> Result<Self, String> {
        Self::with_parts(
            create_headless_engine()?,
            crate::app_paths::AppPaths::in_dir(root),
        )
    }

    /// Initializes GUI state from an audio driver and filesystem locations.
    pub fn with_parts(
        audio_driver: PlayerEngine,
        paths: crate::app_paths::AppPaths,
    ) -> Result<Self, String> {
        // The remote daemon stays off until the user enables it (audit SEC-01).
        let mut remote =
            crate::remote_manager::RemoteGuiManager::new(Some(paths.remote_sessions_file()));
        remote.upload_dir = Some(paths.remote_uploads_dir());
        let config_store = amp_core_engine::ConfigStore::load_or_default(paths.config_file(), None);
        let config = config_store.current();

        let mut app = Self {
            audio_driver,
            playlist: Vec::new(),
            selected_track_idx: 0,
            search_query: String::new(),
            filter_source: None,
            search_query_text: String::new(),
            search_url_text: String::new(),
            focus_location_field: false,
            dialog_job: None,
            media_controls: None,
            library_index: None,
            library_generation: 0,
            library_job: None,
            library_pending: Vec::new(),
            library_ui: crate::library_window::LibraryUi::default(),
            onboarding: None,
            dialog_for_onboarding: false,
            import_job: None,
            update_job: None,
            update_available: None,
            scrub: crate::transport::Scrub::default(),
            search_results: Vec::new(),
            is_searching: false,
            search_status: None,
            playback_notice: None,
            import_notice: None,
            playback_events: None,
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
            config_store,
            config,
            volume: 0.85,
            eq_preamp: 0.0,
            eq_gains: [0.0; 10],
            eq_enabled: true,
            theme: amp_core_engine::WinampTheme::default(),
            skin_bitmaps: std::collections::HashMap::new(),
            skin_cache: crate::skin_texture::SkinTextureCache::default(),
            visualizer_mode: crate::visualizer_widget::VisualizerMode::Spectrum,
            remote,
            remote_requests: Default::default(),
            requested_by: std::collections::HashMap::new(),
            request_ids: std::collections::HashMap::new(),
            lookup_services: None,
            remote_lookup: None,
            last_playing: None,
            paths,
        };

        app.playback_events = app.audio_driver.subscribe_events();
        app.apply_config_to_engine();
        app.load_layout();
        app.load_session();
        Ok(app)
    }

    /// Applies every setting in `self.config` to the live `PlayerEngine` and UI state (`TASK-150`).
    pub fn apply_config_to_engine(&mut self) {
        self.audio_driver
            .set_crossfade_seconds(self.config.audio.crossfade_duration_ms as f32 / 1000.0);
        if let Some(skin_path) = self.config.ui.active_skin_path.clone() {
            let p = Path::new(&skin_path);
            if p.exists() {
                let _ = self.load_skin_from_path(p);
            }
        }
    }

    /// Atomically persists `self.config` through `self.config_store` and applies it to the engine (`TASK-150`).
    pub fn save_and_apply_config(&mut self) {
        let updated = self.config.clone();
        let _ = self.config_store.update(move |cfg| {
            *cfg = updated;
        });
        self.apply_config_to_engine();
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
            self.requested_by = snap.requested_by;

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

    /// True for tracks a phone uploaded (they live in the temporary upload folder).
    fn is_phone_upload(&self, track: &Track) -> bool {
        match (&track.source, &self.remote.upload_dir) {
            (AudioSource::LocalFile { path }, Some(dir)) => Path::new(path).starts_with(dir),
            _ => false,
        }
    }

    /// Persists current playlist and playback configuration to session snapshot.
    pub fn save_session(&self) {
        let snap = SessionPlaylistSnapshot {
            // Phone uploads are deleted on exit, so they never come back with the session.
            tracks: self
                .playlist
                .iter()
                .filter(|t| !self.is_phone_upload(t))
                .cloned()
                .collect(),
            selected_idx: self.selected_track_idx,
            volume: self.volume,
            eq_preamp: self.eq_preamp,
            eq_gains: self.eq_gains,
            eq_enabled: self.eq_enabled,
            deck_assignments: self.deck_assign.clone(),
            dj_hint_done: self.dj_hint_done,
            requested_by: self
                .requested_by
                .iter()
                .filter(|(id, _)| self.playlist.iter().any(|t| t.id == **id))
                .map(|(id, name)| (*id, name.clone()))
                .collect(),
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
        let tracks = amp_adapter_storage::import_m3u8_with_base(&content, src_path.parent());
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
        self.ingest_paths(&[path.to_path_buf()]);
    }

    /// Ingests files and folders as one import: playable audio is added, skins and playlists
    /// are loaded, and audio that cannot be played is summed up in [`Self::import_notice`].
    pub fn ingest_paths(&mut self, paths: &[std::path::PathBuf]) {
        let factory = self.audio_driver.decoder_factory();
        // DSD has no tags lofty reads: name it after the file, with the decoder's length.
        let fallback = move |path: &Path| {
            let dec = factory.open_file(&path.to_string_lossy()).ok()?;
            Some(amp_adapter_storage::file_name_track(
                path,
                dec.duration(),
                Some(dec.sample_rate()),
            ))
        };
        let report = amp_adapter_storage::import::import_paths(paths, &fallback);
        self.remember_in_library(&report.tracks);
        for skin in &report.skins {
            let _ = self.load_skin_from_path(skin);
        }
        self.playlist.extend(report.tracks);
        self.import_notice = crate::playback_notice::skipped_summary(&report.skipped);
        self.save_session();
    }

    /// Scans the configured music folder (normally ~/Music).
    pub fn scan_user_music_dir(&mut self) {
        let folders: Vec<std::path::PathBuf> = if self.config.library.music_directories.is_empty() {
            self.paths
                .music_dir
                .iter()
                .filter(|d| d.is_dir())
                .cloned()
                .collect()
        } else {
            self.config
                .library
                .music_directories
                .iter()
                .map(std::path::PathBuf::from)
                .collect()
        };
        self.start_import(folders);
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
