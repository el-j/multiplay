//! Main application re-exports and integration tests for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

pub use crate::app_render::render_modular_windows;
pub use crate::app_state::MultiplayGuiApp;

#[cfg(test)]
mod tests {
    use super::*;
    use amp_adapter_decoder::AudioStreamDecoder;
    use amp_domain::{AudioSource, Track};
    use std::path::Path;
    use std::time::Duration;

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

        let mut app = MultiplayGuiApp::new().expect("Failed to initialize app");
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
        let mut app = MultiplayGuiApp::new().expect("Failed to initialize app");
        app.playlist.clear();
        assert!(app.playlist.is_empty());
        app.open_file_dialog();
        app.open_folder_dialog();
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
        let mut app = MultiplayGuiApp::new().expect("Failed to initialize app");
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
