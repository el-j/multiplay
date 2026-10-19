//! Main application re-exports and integration tests for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

pub use crate::app_render::render_modular_windows;
pub use crate::app_state::MultiplayGuiApp;

#[cfg(test)]
mod tests {
    use super::*;
    use amp_adapter_decoder::AudioStreamDecoder;
    use amp_domain::{AudioSource, Track};
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    /// Temporary state root removed on drop; keeps tests away from the user's home.
    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let dir =
                std::env::temp_dir().join(format!("multiplay-gui-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).expect("create test root");
            Self(dir)
        }

        fn app(&self) -> MultiplayGuiApp {
            MultiplayGuiApp::headless(&self.0).expect("Failed to initialize headless app")
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn create_test_wav(path: &Path) {
        let sample_rate = 44100u32;
        let num_samples = 512;
        let data_size = (num_samples * 2 * 2) as u32;
        let file_size = 36 + data_size;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * 2 * 2;
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());

        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for i in 0..num_samples {
            let val = ((i as f32 * 0.1).sin() * 32767.0) as i16;
            bytes.extend_from_slice(&val.to_le_bytes());
            bytes.extend_from_slice(&val.to_le_bytes());
        }

        std::fs::write(path, bytes).expect("Failed to write test wav");
    }

    #[test]
    fn test_drag_and_drop() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_drag_drop_song.wav");
        create_test_wav(&test_file);

        let root = TestRoot::new();
        let mut app = root.app();
        let initial_len = app.playlist.len();

        app.ingest_path(&test_file);
        assert_eq!(app.playlist.len(), initial_len + 1);

        app.ingest_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(app.playlist.len(), initial_len + 2);
        assert!(matches!(
            app.playlist[initial_len + 1].source,
            AudioSource::YouTube { .. }
        ));

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_playlist_empty_state() {
        let root = TestRoot::new();
        let mut app = root.app();
        app.playlist.clear();
        assert!(app.playlist.is_empty());
        // Cancelled dialogs must not trigger a library rescan.
        app.open_file_dialog();
        app.open_folder_dialog();
        assert!(app.playlist.is_empty());
        assert!(
            !app.remote.is_running,
            "remote control must be off by default"
        );
    }

    #[test]
    fn test_real_audio_streaming() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_real_stream_song.wav");
        create_test_wav(&test_file);

        let mut decoder = AudioStreamDecoder::open_file(test_file.to_str().unwrap())
            .expect("Failed to open real decoder");
        assert_eq!(decoder.sample_rate(), 44100);

        let mut pcm_buf = vec![0.0f32; 16];
        let read = decoder
            .read_interleaved_samples(&mut pcm_buf)
            .expect("decode failed");
        assert!(read > 0);

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_playback_tracking_and_seek() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_tracking_song.wav");
        create_test_wav(&test_file);

        let mut decoder = AudioStreamDecoder::open_file(test_file.to_str().unwrap())
            .expect("Failed to open real decoder");
        assert_eq!(decoder.elapsed(), Duration::ZERO);

        let mut pcm_buf = vec![0.0f32; 16];
        let _ = decoder.read_interleaved_samples(&mut pcm_buf);
        assert!(decoder.elapsed() >= Duration::ZERO);

        let seek_res = decoder.seek(Duration::from_millis(0));
        assert!(seek_res.is_ok());

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_playlist_m3u8_export_and_import() {
        let root = TestRoot::new();
        let mut app = root.app();
        app.playlist.clear();

        let track = Track::new(
            AudioSource::LocalFile {
                path: "/music/track1.flac".to_string(),
            },
            amp_domain::TrackMetadata::new("Song One", "Artist One"),
        );
        app.playlist.push(track);

        let temp_m3u8 = std::env::temp_dir().join(format!("test_pl_{}.m3u8", uuid::Uuid::new_v4()));
        app.export_playlist_to_m3u8(&temp_m3u8)
            .expect("Export failed");
        assert!(temp_m3u8.exists());

        app.playlist.clear();
        let imported = app
            .import_playlist_from_m3u8(&temp_m3u8)
            .expect("Import failed");
        assert_eq!(imported, 1);
        assert_eq!(app.playlist.len(), 1);
        assert_eq!(app.playlist[0].metadata.title, "Song One");

        let _ = std::fs::remove_file(temp_m3u8);
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn session_is_written_inside_injected_root_only() {
        let root =
            std::env::temp_dir().join(format!("multiplay-gui-persist-{}", uuid::Uuid::new_v4()));
        let mut app = MultiplayGuiApp::headless(&root).expect("headless app");
        app.ingest_url("http://127.0.0.1:1/stream.mp3");
        app.save_layout();
        assert!(app.paths.session_file().exists());
        assert!(app.paths.layout_file.exists());

        let reloaded = MultiplayGuiApp::headless(&root).expect("reload");
        assert_eq!(reloaded.playlist.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn layout_survives_a_restart() {
        use crate::docking::{DockRect, WindowId};
        let root =
            std::env::temp_dir().join(format!("multiplay-gui-layout-{}", uuid::Uuid::new_v4()));
        let mut app = MultiplayGuiApp::headless(&root).expect("headless app");
        let moved = DockRect::new(600.0, 100.0, 360.0, 420.0);
        app.dock_manager.set_rect(WindowId::Playlist, moved);
        app.dock_manager.set_visible(WindowId::Equalizer, false);
        app.applied_zoom = 2.0;
        app.save_layout_if_changed();

        let reloaded = MultiplayGuiApp::headless(&root).expect("reload");
        assert!(!reloaded.dock_manager.is_visible(WindowId::Equalizer));
        assert_eq!(reloaded.startup_zoom, 2.0);
        // Stored at 1x, so the playlist keeps its offset from main and its size.
        let main = reloaded.dock_manager.get_rect(WindowId::Main);
        let pl = reloaded.dock_manager.get_rect(WindowId::Playlist);
        assert_eq!((pl.x - main.x, pl.y - main.y), (480.0, 0.0));
        assert_eq!((pl.width, pl.height), (360.0, 420.0));
        let _ = std::fs::remove_dir_all(root);
    }
}
