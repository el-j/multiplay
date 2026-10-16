//! Pure-Rust Runtime Composition Root for Multiplay.
//! Assembles hexagonal ports, concrete adapters, CQRS event bus, and TUI loop.

#![forbid(unsafe_code)]

use amp_adapter_cpal::CpalAudioSink;
use amp_adapter_p2p::SecurePeerNode;
use amp_adapter_soulseek::SoulseekAdapter;
use amp_adapter_storage::RedbStorageAdapter;
use amp_adapter_tui::{
    DenseWinampConsole, DockingWindowManager, EqualizerWidget, PlaylistWidget, SearchViewModal,
    SearchViewState, WindowId, FACEPLATE_HEIGHT, FACEPLATE_WIDTH,
};
use amp_adapter_youtube::PureYouTubeAdapter;
use amp_core_engine::{EventBus, FilteredSearchCoordinator, PlayerCommand, PlayerEvent};
use amp_domain::source_badge::SourceKind;
use amp_domain::{AudioSource, PlaybackState, Track, TrackMetadata};
use amp_ports::AudioSinkPort;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

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
}

impl MultiplayApp {
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<PlayerCommand>), String> {
        let (event_bus, cmd_rx) = EventBus::new(64);

        // 1. Storage
        let temp_dir = std::env::temp_dir().join(format!("multiplay_db_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("library.redb");
        let storage = Arc::new(
            RedbStorageAdapter::new(&db_path)
                .map_err(|e| format!("Failed to initialize RedbStorageAdapter: {e}"))?,
        );

        // 2. Search Aggregator & Providers
        let mut search_coordinator = FilteredSearchCoordinator::new();

        if let Ok(yt) = PureYouTubeAdapter::new() {
            search_coordinator.register_provider(SourceKind::YouTube, Arc::new(yt));
        }

        let soulseek = Arc::new(SoulseekAdapter::default());
        search_coordinator.register_provider(SourceKind::Soulseek, soulseek);

        let p2p = Arc::new(SecurePeerNode::default());
        search_coordinator.register_provider(SourceKind::P2PShared, p2p);

        // 3. UI Window Manager & Search Dialog
        let docking_mgr = DockingWindowManager::new();
        let search_state = SearchViewState::new();

        // 4. Default Audiophile Playlist
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

        let mut t3 = TrackMetadata::new("Computer World (Pure 24-bit)", "Kraftwerk");
        t3.duration = Some(Duration::from_secs(305));
        t3.bitrate_kbps = Some(2304);
        sample_tracks.push(Track::new(
            AudioSource::YouTube {
                video_id: "dQw4w9WgXcQ".to_string(),
            },
            t3,
        ));

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
        };

        Ok((app, cmd_rx))
    }

    /// Handles a user keyboard event.
    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        // Global quit
        if key == KeyCode::Char('q') && !self.search_state.is_open {
            return true; // Exit app
        }
        if key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            return true; // Exit app
        }

        // Search overlay handling
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
                        self.current_title = format!("{} - {}", track.metadata.artist, track.metadata.title);
                        self.playback_state = PlaybackState::Playing;
                        self.event_bus.send_command(PlayerCommand::PlayTrack(Box::new(track.clone())));
                    }
                    self.search_state.close();
                }
                _ => {}
            }
            return false;
        }

        // Main controls
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
                    self.selected_playlist_idx = (self.selected_playlist_idx + 1).min(self.playlist.len() - 1);
                }
            }
            KeyCode::Enter => {
                if !self.playlist.is_empty() {
                    let track = &self.playlist[self.selected_playlist_idx];
                    self.current_title = format!(
                        "{}. {} - {}",
                        self.selected_playlist_idx + 1,
                        track.metadata.artist,
                        track.metadata.title
                    );
                    self.playback_state = PlaybackState::Playing;
                    self.event_bus.send_command(PlayerCommand::PlayTrack(Box::new(track.clone())));
                }
            }
            KeyCode::Char('e') => {
                self.docking_mgr.toggle_tear_off(WindowId::Equalizer);
            }
            KeyCode::Char('p') => {
                self.docking_mgr.toggle_tear_off(WindowId::Playlist);
            }
            KeyCode::Char(' ') | KeyCode::Char('x') => {
                // Play / Pause toggle
                if self.playback_state == PlaybackState::Playing {
                    self.playback_state = PlaybackState::Paused;
                } else {
                    self.playback_state = PlaybackState::Playing;
                }
                self.event_bus.send_command(PlayerCommand::TogglePause);
            }
            KeyCode::Char('v') => {
                self.playback_state = PlaybackState::Stopped;
                self.elapsed = Duration::ZERO;
                self.event_bus.send_command(PlayerCommand::Stop);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.volume = (self.volume + 5).min(100);
                self.event_bus.send_command(PlayerCommand::SetVolume(self.volume as f32 / 100.0));
            }
            KeyCode::Char('-') => {
                self.volume = self.volume.saturating_sub(5);
                self.event_bus.send_command(PlayerCommand::SetVolume(self.volume as f32 / 100.0));
            }
            KeyCode::Char('w') if modifiers.contains(KeyModifiers::ALT) => {
                self.docking_mgr.toggle_tear_off(WindowId::Equalizer);
            }
            _ => {}
        }

        false
    }

    /// Renders current frame to terminal buffer.
    pub fn render_frame(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // 1. Main Winamp faceplate
        if let Some(main_win) = self.docking_mgr.get_window(WindowId::MainConsole) {
            if main_win.visible && area.width >= FACEPLATE_WIDTH && area.height >= FACEPLATE_HEIGHT {
                let console = DenseWinampConsole::new(&self.spectrum)
                    .title(&self.current_title)
                    .elapsed(self.elapsed)
                    .state(self.playback_state)
                    .volume(self.volume)
                    .audio_info(44.1, 320, true);
                frame.render_widget(console, main_win.rect);
            }
        }

        // 2. Equalizer docked window
        if let Some(eq_win) = self.docking_mgr.get_window(WindowId::Equalizer) {
            if eq_win.visible && area.height >= 13 && area.width >= FACEPLATE_WIDTH {
                let eq_widget = EqualizerWidget::new(&self.eq_gains, self.eq_enabled, 0.0);
                frame.render_widget(eq_widget, eq_win.rect);
            }
        }

        // 3. Playlist docked window
        if let Some(pl_win) = self.docking_mgr.get_window(WindowId::Playlist) {
            if pl_win.visible && area.height >= 19 && area.width >= FACEPLATE_WIDTH {
                let total_secs: u64 = self
                    .playlist
                    .iter()
                    .filter_map(|t| t.metadata.duration)
                    .map(|d| d.as_secs())
                    .sum();
                let pl_widget = PlaylistWidget::new(&self.playlist, self.selected_playlist_idx, total_secs);
                frame.render_widget(pl_widget, pl_win.rect);
            }
        }

        // 4. Search Modal Overlay (if open)
        if self.search_state.is_open {
            let modal = SearchViewModal::new(&self.search_state);
            frame.render_widget(modal, area);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut cmd_rx) = MultiplayApp::new().map_err(|e| format!("Initialization error: {e}"))?;

    // Shared state between TUI loop and real-time audio playback worker
    let is_playing_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let visualizer_spectrum = Arc::new(std::sync::Mutex::new([0.0f32; 16]));
    let volume_ref = Arc::new(std::sync::Mutex::new(0.85f32));

    // Initialize CPAL audio sink and keep alive for duration of main loop
    let mut _active_audio_sink = None;
    if let Ok((mut audio_sink, mut producer)) = CpalAudioSink::new() {
        let sample_rate = audio_sink.sample_rate();
        let _ = audio_sink.start();

        let play_flag = Arc::clone(&is_playing_flag);
        let spec_out = Arc::clone(&visualizer_spectrum);
        let vol_out = Arc::clone(&volume_ref);

        std::thread::Builder::new()
            .name("tui-audio-feeder".to_string())
            .spawn(move || {
                let mut vis = amp_core_engine::SpectrumVisualizer::new();
                let chunk_frames = 512;
                let mut pcm_buffer = vec![0.0f32; chunk_frames * 2];
                let mut phase: f32 = 0.0;
                let mut phase2: f32 = 0.0;
                let notes = [220.0, 261.63, 329.63, 392.0, 440.0, 523.25, 659.25, 783.99];
                let mut note_step = 0;

                while !std::thread::panicking() {
                    if !play_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    }

                    let vol = *vol_out.lock().unwrap();
                    let target_freq = notes[(note_step / 16) % notes.len()];
                    note_step += 1;

                    #[allow(clippy::chunks_exact_to_as_chunks)]
                    for frame in pcm_buffer.chunks_exact_mut(2) {
                        let osc1 = (phase * 2.0 * std::f32::consts::PI).sin() * 0.3;
                        let osc2 = (phase2 * 2.0 * std::f32::consts::PI).sin() * 0.15;
                        frame[0] = (osc1 + osc2) * vol;
                        frame[1] = (osc1 * 0.9 + osc2 * 1.1) * vol;

                        phase += target_freq / sample_rate as f32;
                        if phase > 1.0 { phase -= 1.0; }
                        phase2 += (target_freq * 1.5) / sample_rate as f32;
                        if phase2 > 1.0 { phase2 -= 1.0; }
                    }

                    vis.push_samples(&pcm_buffer);
                    let bands = vis.compute_spectrum_bands();
                    if let Ok(mut spec) = spec_out.try_lock() {
                        *spec = bands;
                    }

                    for &s in &pcm_buffer {
                        while producer.push(s).is_err() {
                            if !play_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                break;
                            }
                            std::thread::yield_now();
                        }
                    }
                }
            })
            .ok();

        _active_audio_sink = Some(audio_sink);
    }

    // Spawn command processing loop
    let bus_clone = EventBus {
        cmd_sender: app.event_bus.cmd_sender.clone(),
        event_broadcaster: app.event_bus.event_broadcaster.clone(),
    };
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                PlayerCommand::TogglePause => {
                    bus_clone.emit_event(PlayerEvent::PlaybackStateChanged(PlaybackState::Paused));
                }
                PlayerCommand::Stop => {
                    bus_clone.emit_event(PlayerEvent::PlaybackStateChanged(PlaybackState::Stopped));
                }
                _ => {}
            }
        }
    });

    // Check if running in a terminal TTY
    if !crossterm::tty::IsTty::is_tty(&stdout()) {
        println!("Multiplay pure-Rust engine initialized successfully. Non-interactive mode terminating.");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut event_rx = app.event_bus.subscribe();

    loop {
        // Synchronize audio playback state and volume with real-time audio thread
        is_playing_flag.store(app.playback_state == PlaybackState::Playing, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut vol) = volume_ref.try_lock() {
            *vol = app.volume as f32 / 100.0;
        }
        if let Ok(spec) = visualizer_spectrum.try_lock() {
            app.spectrum = *spec;
        }

        // Poll event bus
        while let Ok(event) = event_rx.try_recv() {
            if let PlayerEvent::PlaybackStateChanged(s) = event {
                app.playback_state = s;
            }
        }

        // Render frame
        terminal.draw(|f| app.render_frame(f))?;

        // Poll keyboard input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                if app.handle_key(key_event.code, key_event.modifiers) {
                    break; // Exit
                }
            }
        }

        // Advance playback elapsed time if playing
        if app.playback_state == PlaybackState::Playing {
            app.elapsed += Duration::from_millis(50);
        }
    }

    // Clean terminal shutdown
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiplay_app_initialization_and_keybindings() {
        let (mut app, _cmd_rx) = MultiplayApp::new().expect("Failed to initialize MultiplayApp");
        assert_eq!(app.playback_state, PlaybackState::Stopped);
        assert_eq!(app.volume, 85);

        // Test spacebar toggles play
        app.handle_key(KeyCode::Char(' '), KeyModifiers::empty());
        assert_eq!(app.playback_state, PlaybackState::Playing);

        // Test spacebar pauses
        app.handle_key(KeyCode::Char(' '), KeyModifiers::empty());
        assert_eq!(app.playback_state, PlaybackState::Paused);

        // Test stop key 'v'
        app.handle_key(KeyCode::Char('v'), KeyModifiers::empty());
        assert_eq!(app.playback_state, PlaybackState::Stopped);

        // Test volume controls
        app.handle_key(KeyCode::Char('+'), KeyModifiers::empty());
        assert_eq!(app.volume, 90);
        app.handle_key(KeyCode::Char('-'), KeyModifiers::empty());
        assert_eq!(app.volume, 85);

        // Test search dialog open and close
        assert!(!app.search_state.is_open);
        app.handle_key(KeyCode::Char('j'), KeyModifiers::empty());
        assert!(app.search_state.is_open);
        app.handle_key(KeyCode::Esc, KeyModifiers::empty());
        assert!(!app.search_state.is_open);

        // Test playlist navigation
        assert_eq!(app.selected_playlist_idx, 0);
        app.handle_key(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(app.selected_playlist_idx, 1);
        app.handle_key(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(app.playback_state, PlaybackState::Playing);
        assert!(app.current_title.contains("Boards of Canada"));

        // Test EQ toggle
        app.handle_key(KeyCode::Char('e'), KeyModifiers::empty());

        // Test quit
        let should_exit = app.handle_key(KeyCode::Char('q'), KeyModifiers::empty());
        assert!(should_exit);
    }
}
