//! Pure-Rust Runtime Composition Root for Multiplay.
//! Assembles hexagonal ports, audio playback engine, CQRS event bus, and TUI loop.

#![forbid(unsafe_code)]

pub mod app_state;
pub mod audio_driver;
pub mod runner;
pub mod terminal;
pub mod track_loader;

pub use app_state::MultiplayApp;
pub use audio_driver::AudioPlaybackDriver;

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::PlaybackState;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_multiplay_app_initialization_and_keybindings() {
        let (mut app, _cmd_rx) =
            MultiplayApp::new_headless().expect("Failed to initialize MultiplayApp");
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
