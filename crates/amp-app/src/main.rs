//! Pure-Rust Runtime Composition Root for Multiplay.
//! Assembles hexagonal ports, concrete adapters, CQRS event bus, and TUI loop.

#![forbid(unsafe_code)]

use amp_adapter_cpal::CpalAudioSink;
use amp_adapter_p2p::SecurePeerNode;
use amp_adapter_soulseek::SoulseekAdapter;
use amp_adapter_storage::RedbStorageAdapter;
use amp_adapter_tui::{
    DenseWinampConsole, DockingWindowManager, SearchViewModal, SearchViewState, WindowId,
    FACEPLATE_HEIGHT, FACEPLATE_WIDTH,
};
use amp_adapter_youtube::PureYouTubeAdapter;
use amp_core_engine::{EventBus, FilteredSearchCoordinator, PlayerCommand, PlayerEvent};
use amp_domain::source_badge::SourceKind;
use amp_domain::PlaybackState;
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

        let app = Self {
            event_bus,
            storage,
            search_coordinator,
            docking_mgr,
            search_state,
            spectrum: [0.0; 16],
            playback_state: PlaybackState::Stopped,
            current_title: "1. Multiplay Pure-Rust Audiophile Player".to_string(),
            elapsed: Duration::ZERO,
            volume: 85,
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

        // 2. Search Modal Overlay (if open)
        if self.search_state.is_open {
            let modal = SearchViewModal::new(&self.search_state);
            frame.render_widget(modal, area);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut cmd_rx) = MultiplayApp::new().map_err(|e| format!("Initialization error: {e}"))?;

    // Initialize CPAL audio sink
    if let Ok((mut audio_sink, _producer)) = CpalAudioSink::new() {
        let _ = audio_sink.configure(44100, 2);
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

        // Test quit
        let should_exit = app.handle_key(KeyCode::Char('q'), KeyModifiers::empty());
        assert!(should_exit);
    }
}
