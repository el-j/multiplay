//! Multiplay application state, commands, and TUI view models.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use amp_adapter_p2p::SecurePeerNode;
use amp_adapter_soulseek::SoulseekAdapter;
use amp_adapter_storage::RedbStorageAdapter;
use amp_adapter_tui::{
    DenseWinampConsole, DockingWindowManager, EqualizerWidget, PlaylistWidget, SearchViewModal,
    SearchViewState, WindowId, FACEPLATE_HEIGHT, FACEPLATE_WIDTH,
};
use amp_adapter_youtube::PureYouTubeAdapter;
use amp_core_engine::{EventBus, FilteredSearchCoordinator, PlayerCommand};
use amp_domain::source_badge::SourceKind;
use amp_domain::{AudioSource, PlaybackState, Track, TrackMetadata};
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Core TUI application state holding models, active window manager, and audio driver.
pub struct MultiplayApp {
    pub event_bus: EventBus,
    pub storage: Arc<RedbStorageAdapter>,
    pub search_coordinator: FilteredSearchCoordinator,
    pub docking_mgr: DockingWindowManager,
    pub search_state: SearchViewState,
    pub spectrum: [f32; 16],
    pub playback_state: PlaybackState,
    pub current_title: String,
    pub elapsed: Duration,
    pub volume: u8,
    pub eq_gains: [f32; 10],
    pub eq_enabled: bool,
    pub playlist: Vec<Track>,
    pub selected_playlist_idx: usize,
    pub audio_driver: Option<Arc<AudioPlaybackDriver>>,
}

impl MultiplayApp {
    /// Creates a new MultiplayApp with real storage, search providers, and audio driver.
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<PlayerCommand>), String> {
        Self::with_driver(AudioPlaybackDriver::new().ok())
    }

    /// Creates a MultiplayApp that plays into a null sink (tests and CI; no audio hardware).
    pub fn new_headless() -> Result<(Self, mpsc::UnboundedReceiver<PlayerCommand>), String> {
        Self::with_driver(AudioPlaybackDriver::new_headless().ok())
    }

    fn with_driver(
        audio_driver: Option<AudioPlaybackDriver>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<PlayerCommand>), String> {
        let (event_bus, cmd_rx) = EventBus::new(64);

        let temp_dir = std::env::temp_dir().join(format!("multiplay_db_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("library.redb");
        let storage = Arc::new(
            RedbStorageAdapter::new(&db_path)
                .map_err(|e| format!("Failed to initialize RedbStorageAdapter: {e}"))?,
        );

        let mut search_coordinator = FilteredSearchCoordinator::new();
        if let Ok(yt) = PureYouTubeAdapter::new() {
            search_coordinator.register_provider(SourceKind::YouTube, Arc::new(yt));
        }
        let soulseek = Arc::new(SoulseekAdapter::default());
        search_coordinator.register_provider(SourceKind::Soulseek, soulseek);
        let p2p = Arc::new(SecurePeerNode::default());
        search_coordinator.register_provider(SourceKind::P2PShared, p2p);

        let docking_mgr = DockingWindowManager::new();
        let search_state = SearchViewState::new();

        let mut sample_tracks = Vec::new();
        let mut t1 = TrackMetadata::new("Polynomial-C", "Aphex Twin");
        t1.duration = Some(Duration::from_secs(286));
        t1.bitrate_kbps = Some(320);
        sample_tracks.push(Track::new(
            AudioSource::Soulseek {
                username: "peer_audiophile".to_string(),
                filename: "Aphex Twin - Polynomial-C.mp3".to_string(),
            },
            t1,
        ));

        let mut t2 = TrackMetadata::new("Music Is Math", "Boards of Canada");
        t2.duration = Some(Duration::from_secs(321));
        t2.bitrate_kbps = Some(1411);
        sample_tracks.push(Track::new(
            AudioSource::LocalFile {
                path: "music/boc_music_is_math.flac".to_string(),
            },
            t2,
        ));

        let audio_driver = audio_driver.map(Arc::new);

        let app = Self {
            event_bus,
            storage,
            search_coordinator,
            docking_mgr,
            search_state,
            spectrum: [0.0; 16],
            playback_state: PlaybackState::Stopped,
            current_title: "1. Aphex Twin - Polynomial-C".to_string(),
            elapsed: Duration::ZERO,
            volume: 85,
            eq_gains: [0.0; 10],
            eq_enabled: true,
            playlist: sample_tracks,
            selected_playlist_idx: 0,
            audio_driver,
        };

        Ok((app, cmd_rx))
    }

    /// Handles a user keyboard event. Returns true when application should exit.
    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if key == KeyCode::Char('q') && !self.search_state.is_open {
            return true;
        }
        if key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }

        if self.search_state.is_open {
            match key {
                KeyCode::Esc => self.search_state.close(),
                KeyCode::Tab => self.search_state.cycle_filter_pill(),
                KeyCode::Down => self.search_state.select_next(),
                KeyCode::Up => self.search_state.select_prev(),
                KeyCode::Char(c) => self.search_state.input_query.push(c),
                KeyCode::Backspace => {
                    self.search_state.input_query.pop();
                }
                KeyCode::Enter => {
                    if let Some(track) = self.search_state.selected_track() {
                        self.current_title =
                            format!("{} - {}", track.metadata.artist, track.metadata.title);
                        self.playback_state = PlaybackState::Playing;
                        if let Some(ref driver) = self.audio_driver {
                            driver.play(track.clone());
                        }
                        self.event_bus
                            .send_command(PlayerCommand::PlayTrack(Box::new(track.clone())));
                    }
                    self.search_state.close();
                }
                _ => {}
            }
            return false;
        }

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
            KeyCode::Enter => {
                if !self.playlist.is_empty() {
                    let track = self.playlist[self.selected_playlist_idx].clone();
                    self.current_title = format!(
                        "{}. {} - {}",
                        self.selected_playlist_idx + 1,
                        track.metadata.artist,
                        track.metadata.title
                    );
                    self.playback_state = PlaybackState::Playing;
                    if let Some(ref driver) = self.audio_driver {
                        driver.play(track.clone());
                    }
                    self.event_bus
                        .send_command(PlayerCommand::PlayTrack(Box::new(track)));
                }
            }
            KeyCode::Char('e') => {
                self.docking_mgr.toggle_tear_off(WindowId::Equalizer);
            }
            KeyCode::Char('p') => {
                self.docking_mgr.toggle_tear_off(WindowId::Playlist);
            }
            KeyCode::Char(' ') | KeyCode::Char('x') => {
                if self.playback_state == PlaybackState::Playing {
                    self.playback_state = PlaybackState::Paused;
                } else {
                    self.playback_state = PlaybackState::Playing;
                }
                if let Some(ref driver) = self.audio_driver {
                    driver.toggle_play_pause();
                }
                self.event_bus.send_command(PlayerCommand::TogglePause);
            }
            KeyCode::Char('v') => {
                self.playback_state = PlaybackState::Stopped;
                self.elapsed = Duration::ZERO;
                if let Some(ref driver) = self.audio_driver {
                    driver.stop();
                }
                self.event_bus.send_command(PlayerCommand::Stop);
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
            KeyCode::Char('w') if modifiers.contains(KeyModifiers::ALT) => {
                self.docking_mgr.toggle_tear_off(WindowId::Equalizer);
            }
            _ => {}
        }

        false
    }

    /// Synchronizes playback driver state (spectrum, elapsed duration) into UI view model.
    pub fn sync_audio_state(&mut self) {
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
                let console = DenseWinampConsole::new(&self.spectrum)
                    .title(&self.current_title)
                    .elapsed(self.elapsed)
                    .state(self.playback_state)
                    .volume(self.volume)
                    .audio_info(44.1, 320, true);
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
    }
}
