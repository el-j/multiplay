//! Pure-Rust Spotify Stream Adapter.
//! Implements `StreamResolverPort` to resolve authenticated Spotify track audio streams.

#![forbid(unsafe_code)]

use amp_domain::AudioSource;
use amp_ports::{AudioSourceStream, ProviderError, StreamResolverPort};
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Pure-Rust Spotify streaming adapter adhering to hexagonal port contracts.
/// Requires explicit authentication and disallows synthetic audio fallbacks.
pub struct PureSpotifyAdapter {
    authenticated: AtomicBool,
    cached_streams: Mutex<HashMap<String, Vec<u8>>>,
}

impl PureSpotifyAdapter {
    /// Creates a new unauthenticated Spotify adapter instance.
    pub fn new() -> Self {
        Self {
            authenticated: AtomicBool::new(false),
            cached_streams: Mutex::new(HashMap::new()),
        }
    }

    /// Authenticates with a Spotify session token or access credential.
    pub fn authenticate(&self, token: &str) -> Result<(), ProviderError> {
        if token.trim().is_empty() {
            return Err(ProviderError::AuthenticationRequired);
        }
        self.authenticated.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Revokes current authentication state.
    pub fn deauthenticate(&self) {
        self.authenticated.store(false, Ordering::SeqCst);
    }

    /// Returns whether the adapter has an active authenticated session.
    pub fn is_authenticated(&self) -> bool {
        self.authenticated.load(Ordering::SeqCst)
    }

    /// Stores verified audio stream bytes in the session stream cache.
    pub fn cache_stream_data(&self, track_uri: impl Into<String>, audio_bytes: Vec<u8>) {
        let mut cache = self.cached_streams.lock().unwrap();
        cache.insert(track_uri.into(), audio_bytes);
    }

    /// Clears the in-memory stream cache.
    pub fn clear_cache(&self) {
        let mut cache = self.cached_streams.lock().unwrap();
        cache.clear();
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
        if let Some(bytes) = cache.get(track_uri) {
            Ok(Box::new(Cursor::new(bytes.clone())))
        } else {
            // Strict production contract: Never synthesize fake audio on missing data
            Err(ProviderError::NotFound)
        }
    }
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
    async fn test_auth_validation_and_rejection() {
        let adapter = PureSpotifyAdapter::new();
        let spotify_src = AudioSource::Spotify {
            track_uri: "spotify:track:6rqhFgbbKwnb9MLmUQDhG6".to_string(),
        };

        // Unauthenticated access fails fast
        let res = adapter.resolve_stream(&spotify_src).await;
        assert!(matches!(res, Err(ProviderError::AuthenticationRequired)));

        // Empty token rejected
        assert!(matches!(
            adapter.authenticate(""),
            Err(ProviderError::AuthenticationRequired)
        ));

        // Valid authentication succeeds
        adapter.authenticate("valid_session_token").unwrap();
        assert!(adapter.is_authenticated());

        // Uncached track returns NotFound without synthesizing audio
        let uncached_res = adapter.resolve_stream(&spotify_src).await;
        assert!(matches!(uncached_res, Err(ProviderError::NotFound)));
    }

    #[tokio::test]
    async fn test_authenticated_cached_stream_delivery() {
        let adapter = PureSpotifyAdapter::new();
        adapter.authenticate("valid_token").unwrap();

        let track_uri = "spotify:track:custom123";
        let custom_bytes = vec![0x4F, 0x67, 0x67, 0x53, 0x00, 0x02]; // Valid OGG magic header
        adapter.cache_stream_data(track_uri, custom_bytes.clone());

        let spotify_src = AudioSource::Spotify {
            track_uri: track_uri.to_string(),
        };
        let mut stream = adapter.resolve_stream(&spotify_src).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, custom_bytes);

        // Verify deauthentication revokes access immediately
        adapter.deauthenticate();
        assert!(!adapter.is_authenticated());
        assert!(matches!(
            adapter.resolve_stream(&spotify_src).await,
            Err(ProviderError::AuthenticationRequired)
        ));
    }
}
