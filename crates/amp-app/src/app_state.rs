//! Multiplay application state, commands, and TUI view models.

#![forbid(unsafe_code)]

use crate::engine_factory::{create_headless_engine, create_player_engine};
use amp_adapter_p2p::SecurePeerNode;
use amp_adapter_soulseek::SoulseekAdapter;
use amp_adapter_storage::RedbStorageAdapter;
use amp_adapter_tui::{
    DenseConsole, DockingWindowManager, EqualizerWidget, PlaylistWidget, SearchViewModal,
    SearchViewState, WindowId, FACEPLATE_HEIGHT, FACEPLATE_WIDTH,
};
use amp_adapter_youtube::PureYouTubeAdapter;
use amp_core_engine::{EventBus, FilteredSearchCoordinator, PlayerCommand, PlayerEngine};
use amp_domain::source_badge::SourceKind;
use amp_domain::{PlaybackState, Track};
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Core TUI application state holding models, active window manager, and audio driver.
pub struct MultiplayApp {
    pub event_bus: EventBus,
    pub storage: Arc<RedbStorageAdapter>,
    pub search_coordinator: Arc<FilteredSearchCoordinator>,
    pub docking_mgr: DockingWindowManager,
    pub search_state: SearchViewState,
    pub search_tx: mpsc::UnboundedSender<Vec<Track>>,
    pub search_rx: mpsc::UnboundedReceiver<Vec<Track>>,
    pub spectrum: [f32; 16],
    pub playback_state: PlaybackState,
    pub current_title: String,
    pub elapsed: Duration,
    pub volume: u8,
    pub eq_gains: [f32; 10],
    pub eq_enabled: bool,
    pub playlist: Vec<Track>,
    pub selected_playlist_idx: usize,
    pub audio_driver: Option<Arc<PlayerEngine>>,
    /// The add prompt (`o`), while open.
    pub prompt: Option<crate::tui_keys::PathPrompt>,
    /// The `?` key overlay.
    pub show_help: bool,
    /// `q` was pressed while playing; `y` quits.
    pub confirm_quit: bool,
    /// EQ band the `{` / `}` keys change.
    pub eq_band: usize,
    /// One-line result of the last action ("Added 12 tracks").
    pub status: Option<String>,
    /// MPRIS media keys and Now Playing (Linux; see `amp-adapter-media-controls`).
    pub media_controls: Option<Box<dyn amp_ports::MediaControlsPort>>,
}

impl MultiplayApp {
    /// Creates a new MultiplayApp with real storage, search providers, and audio driver.
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<PlayerCommand>), String> {
        Self::with_driver(create_player_engine().ok())
    }

    /// Creates a MultiplayApp that plays into a null sink (tests and CI; no audio hardware).
    pub fn new_headless() -> Result<(Self, mpsc::UnboundedReceiver<PlayerCommand>), String> {
        Self::with_driver(create_headless_engine().ok())
    }

    fn with_driver(
        audio_driver: Option<PlayerEngine>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<PlayerCommand>), String> {
        let (event_bus, cmd_rx) = EventBus::new(64);

        let app_paths = amp_adapter_storage::AppPaths::from_env();
        let _ = app_paths.ensure_dirs();
        let storage_adapter = RedbStorageAdapter::open_default(&app_paths).or_else(|_| {
            let temp_dir =
                std::env::temp_dir().join(format!("multiplay_db_{}", uuid::Uuid::new_v4()));
            let _ = std::fs::create_dir_all(&temp_dir);
            RedbStorageAdapter::new(temp_dir.join("library.redb"))
        });
        let storage = Arc::new(
            storage_adapter.map_err(|e| format!("Failed to initialize RedbStorageAdapter: {e}"))?,
        );

        let mut search_coordinator = FilteredSearchCoordinator::new();
        if let Ok(yt) = PureYouTubeAdapter::new() {
            search_coordinator.register_provider(SourceKind::YouTube, Arc::new(yt));
        }
        if let Ok(bc) = amp_adapter_bandcamp::PureBandcampAdapter::new() {
            search_coordinator.register_provider(SourceKind::Bandcamp, Arc::new(bc));
        }
        let soulseek = Arc::new(SoulseekAdapter::default());
        search_coordinator.register_provider(SourceKind::Soulseek, soulseek);
        let p2p = Arc::new(SecurePeerNode::default());
        search_coordinator.register_provider(SourceKind::P2PShared, p2p);

        let (search_tx, search_rx) = mpsc::unbounded_channel();
        let search_coordinator = Arc::new(search_coordinator);

        let docking_mgr = DockingWindowManager::new();
        let search_state = SearchViewState::new();

        let audio_driver = audio_driver.map(Arc::new);

        let app = Self {
            event_bus,
            storage,
            search_coordinator,
            docking_mgr,
            search_state,
            search_tx,
            search_rx,
            spectrum: [0.0; 16],
            playback_state: PlaybackState::Stopped,
            current_title: "No track loaded. Press J to search.".to_string(),
            elapsed: Duration::ZERO,
            volume: 85,
            eq_gains: [0.0; 10],
            eq_enabled: true,
            playlist: Vec::new(),
            selected_playlist_idx: 0,
            audio_driver,
            prompt: None,
            show_help: false,
            confirm_quit: false,
            eq_band: 0,
            status: None,
            media_controls: None,
        };

        Ok((app, cmd_rx))
    }

    /// Handles a user keyboard event. Returns true when application should exit.
    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }
        if self.confirm_quit {
            self.confirm_quit = false;
            return matches!(key, KeyCode::Char('y') | KeyCode::Char('Y'));
        }
        if self.handle_prompt_key(key) {
            return false;
        }
        if self.show_help {
            self.show_help = false;
            return false;
        }

        if self.search_state.is_open {
            match key {
                KeyCode::Esc => self.search_state.close(),
                KeyCode::Tab => self.search_state.cycle_filter_pill(),
                KeyCode::Down => self.search_state.select_next(),
                KeyCode::Up => self.search_state.select_prev(),
                KeyCode::Char('a') if !self.search_state.results.is_empty() => {
                    if let Some(track) = self.search_state.selected_track() {
                        self.playlist.push(track.clone());
                    }
                }
                KeyCode::Char(c) => {
                    self.search_state.input_query.push(c);
                    self.search_state.results.clear();
                }
                KeyCode::Backspace => {
                    self.search_state.input_query.pop();
                    self.search_state.results.clear();
                }
                KeyCode::Enter => {
                    if !self.search_state.results.is_empty() {
                        if let Some(track) = self.search_state.selected_track() {
                            self.playlist.push(track.clone());
                            self.selected_playlist_idx = self.playlist.len() - 1;
                            self.current_title = format!(
                                "{}. {} - {}",
                                self.playlist.len(),
                                track.metadata.artist,
                                track.metadata.title
                            );
                            self.playback_state = PlaybackState::Playing;
                            if let Some(ref driver) = self.audio_driver {
                                driver.play(track.clone());
                            }
                            self.event_bus
                                .send_command(PlayerCommand::PlayTrack(Box::new(track.clone())));
                        }
                        self.search_state.close();
                    } else if !self.search_state.input_query.trim().is_empty() {
                        self.trigger_search();
                    }
                }
                _ => {}
            }
            return false;
        }

        if key == KeyCode::Char('w') && modifiers.contains(KeyModifiers::ALT) {
            self.docking_mgr.toggle_tear_off(WindowId::Equalizer);
            return false;
        }
        self.handle_player_key(key)
    }

    /// Keys the TUI had before TASK-174: search, selection, windows, volume.
    pub(crate) fn handle_classic_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('j') | KeyCode::Char('/') => {
                self.search_state.open();
            }
            KeyCode::Up => {
                if !self.playlist.is_empty() {
                    self.selected_playlist_idx = self.selected_playlist_idx.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if !self.playlist.is_empty() {
                    self.selected_playlist_idx =
                        (self.selected_playlist_idx + 1).min(self.playlist.len() - 1);
                }
            }
            KeyCode::Enter => self.play_index(self.selected_playlist_idx),
            KeyCode::Char('e') => {
                self.docking_mgr.toggle_tear_off(WindowId::Equalizer);
            }
            KeyCode::Char('p') => {
                self.docking_mgr.toggle_tear_off(WindowId::Playlist);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.volume = (self.volume + 5).min(100);
                if let Some(ref driver) = self.audio_driver {
                    driver.set_volume(self.volume as f32 / 100.0);
                }
                self.event_bus
                    .send_command(PlayerCommand::SetVolume(self.volume as f32 / 100.0));
            }
            KeyCode::Char('-') => {
                self.volume = self.volume.saturating_sub(5);
                if let Some(ref driver) = self.audio_driver {
                    driver.set_volume(self.volume as f32 / 100.0);
                }
                self.event_bus
                    .send_command(PlayerCommand::SetVolume(self.volume as f32 / 100.0));
            }
            _ => {}
        }

        false
    }

    /// Spawns background multi-source search across enabled providers (`TASK-153`).
    pub fn trigger_search(&mut self) {
        let q = self.search_state.input_query.trim().to_string();
        if q.is_empty() {
            return;
        }
        let coordinator = Arc::clone(&self.search_coordinator);
        let mask = self.search_state.filter_mask.clone();
        let tx = self.search_tx.clone();
        tokio::spawn(async move {
            use amp_ports::driving::{FilteredSearchPort, SearchQueryOptions};
            let results = coordinator
                .search(SearchQueryOptions {
                    text: q,
                    filter_mask: mask,
                    timeout_millis: 10_000,
                })
                .await;
            let _ = tx.send(results);
        });
    }

    /// Synchronizes playback driver state (spectrum, elapsed duration) into UI view model.
    pub fn sync_audio_state(&mut self) {
        while let Ok(res) = self.search_rx.try_recv() {
            self.search_state.results = res;
            self.search_state.selected_result_idx = 0;
        }

        if let Some(ref driver) = self.audio_driver {
            if let Ok(spec) = driver.visualizer_spectrum.try_lock() {
                self.spectrum = *spec;
            }
            if let Ok(el) = driver.elapsed.try_lock() {
                self.elapsed = *el;
            }
            if driver.is_playing.load(Ordering::Relaxed) {
                self.playback_state = PlaybackState::Playing;
            } else if self.playback_state == PlaybackState::Playing {
                self.playback_state = PlaybackState::Paused;
            }
        }
    }

    /// Renders current frame to terminal buffer.
    pub fn render_frame(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        if let Some(main_win) = self.docking_mgr.get_window(WindowId::MainConsole) {
            if main_win.visible && area.width >= FACEPLATE_WIDTH && area.height >= FACEPLATE_HEIGHT
            {
                let mut console = DenseConsole::new(&self.spectrum)
                    .title(&self.current_title)
                    .elapsed(self.elapsed)
                    .state(self.playback_state)
                    .volume(self.volume);
                if let Some(driver) = &self.audio_driver {
                    if let Some(info) = driver.stream_info() {
                        let meta = driver
                            .current_track
                            .lock()
                            .ok()
                            .and_then(|t| t.as_ref().map(|t| t.metadata.clone()));
                        let kbps = meta.as_ref().and_then(|m| m.bitrate_kbps);
                        let stereo = meta.and_then(|m| m.channels).is_none_or(|c| c >= 2);
                        console =
                            console.audio_info(info.source_rate as f32 / 1000.0, kbps, stereo);
                    }
                }
                frame.render_widget(console, main_win.rect);
            }
        }

        if let Some(eq_win) = self.docking_mgr.get_window(WindowId::Equalizer) {
            if eq_win.visible && area.height >= 13 && area.width >= FACEPLATE_WIDTH {
                let eq_widget = EqualizerWidget::new(&self.eq_gains, self.eq_enabled, 0.0);
                frame.render_widget(eq_widget, eq_win.rect);
            }
        }

        if let Some(pl_win) = self.docking_mgr.get_window(WindowId::Playlist) {
            if pl_win.visible && area.height >= 19 && area.width >= FACEPLATE_WIDTH {
                let total_secs: u64 = self
                    .playlist
                    .iter()
                    .filter_map(|t| t.metadata.duration)
                    .map(|d| d.as_secs())
                    .sum();
                let pl_widget =
                    PlaylistWidget::new(&self.playlist, self.selected_playlist_idx, total_secs);
                frame.render_widget(pl_widget, pl_win.rect);
            }
        }

        if self.search_state.is_open {
            let modal = SearchViewModal::new(&self.search_state);
            frame.render_widget(modal, area);
        }
        self.render_overlays(frame);
    }
}
