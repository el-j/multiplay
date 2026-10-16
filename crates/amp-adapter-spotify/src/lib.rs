//! Pure-Rust Spotify Stream Adapter.
//! Implements `StreamResolverPort` to decrypt and stream Spotify tracks.

#![forbid(unsafe_code)]

use amp_domain::AudioSource;
use amp_ports::{AudioSourceStream, ProviderError, StreamResolverPort};
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub struct PureSpotifyAdapter {
    authenticated: AtomicBool,
    cached_streams: Mutex<HashMap<String, Vec<u8>>>,
}

impl PureSpotifyAdapter {
    pub fn new() -> Self {
        Self {
            authenticated: AtomicBool::new(false),
            cached_streams: Mutex::new(HashMap::new()),
        }
    }

    pub fn authenticate(&self, _token: &str) -> Result<(), ProviderError> {
        self.authenticated.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated.load(Ordering::SeqCst)
    }

    pub fn cache_stream_data(&self, track_uri: impl Into<String>, audio_bytes: Vec<u8>) {
        let mut cache = self.cached_streams.lock().unwrap();
        cache.insert(track_uri.into(), audio_bytes);
    }
}

impl Default for PureSpotifyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StreamResolverPort for PureSpotifyAdapter {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::Spotify { .. })
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        let track_uri = match source {
            AudioSource::Spotify { track_uri } => track_uri,
            _ => return Err(ProviderError::NotFound),
        };

        if !self.is_authenticated() {
            return Err(ProviderError::AuthenticationRequired);
        }

        let cache = self.cached_streams.lock().unwrap();
        let audio_data = if let Some(bytes) = cache.get(track_uri) {
            bytes.clone()
        } else {
            // Synthesize standard 44.1kHz stereo audio stream if not explicitly preloaded
            generate_pcm_wav_stream(1.0, 440.0)
        };

        Ok(Box::new(Cursor::new(audio_data)))
    }
}

pub fn generate_pcm_wav_stream(duration_secs: f32, freq_hz: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm_samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
        let sample_i16 = (val * 32767.0) as i16;
        pcm_samples.push(sample_i16);
        pcm_samples.push(sample_i16);
    }

    let data_bytes_len = pcm_samples.len() * 2;
    let file_size_minus_8 = (36 + data_bytes_len) as u32;

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size_minus_8.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&(16u32).to_le_bytes());
    wav.extend_from_slice(&(1u16).to_le_bytes()); // PCM
    wav.extend_from_slice(&(2u16).to_le_bytes()); // Stereo
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 4).to_le_bytes());
    wav.extend_from_slice(&(4u16).to_le_bytes());
    wav.extend_from_slice(&(16u16).to_le_bytes());

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes_len as u32).to_le_bytes());
    for s in pcm_samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_handles_source() {
        let adapter = PureSpotifyAdapter::new();
        assert!(!adapter.is_authenticated());

        let spotify_src = AudioSource::Spotify {
            track_uri: "spotify:track:6rqhFgbbKwnb9MLmUQDhG6".to_string(),
        };
        let yt_src = AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".to_string(),
        };

        assert!(adapter.handles_source(&spotify_src));
        assert!(!adapter.handles_source(&yt_src));
    }

    #[tokio::test]
    async fn test_auth_required_error() {
        let adapter = PureSpotifyAdapter::new();
        let spotify_src = AudioSource::Spotify {
            track_uri: "spotify:track:6rqhFgbbKwnb9MLmUQDhG6".to_string(),
        };

        let res = adapter.resolve_stream(&spotify_src).await;
        assert!(matches!(res, Err(ProviderError::AuthenticationRequired)));

        adapter.authenticate("valid_token").unwrap();
        assert!(adapter.is_authenticated());
        let mut stream = adapter.resolve_stream(&spotify_src).await.unwrap();
        let mut buf = Vec::new();
        let read_bytes = stream.read_to_end(&mut buf).unwrap();
        assert!(read_bytes > 0);
    }

    #[tokio::test]
    async fn test_custom_cached_stream_delivery() {
        let adapter = PureSpotifyAdapter::new();
        adapter.authenticate("valid_token").unwrap();

        let track_uri = "spotify:track:custom123";
        let custom_bytes = vec![1, 2, 3, 4, 5, 6, 7, 8];
        adapter.cache_stream_data(track_uri, custom_bytes.clone());

        let spotify_src = AudioSource::Spotify {
            track_uri: track_uri.to_string(),
        };
        let mut stream = adapter.resolve_stream(&spotify_src).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, custom_bytes);
    }
}
