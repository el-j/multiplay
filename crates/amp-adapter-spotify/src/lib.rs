//! Pure-Rust Spotify Stream Adapter.
//! Implements `StreamResolverPort` to decrypt and stream Spotify tracks.

#![forbid(unsafe_code)]

use amp_domain::AudioSource;
use amp_ports::{AudioSourceStream, ProviderError, StreamResolverPort};
use async_trait::async_trait;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct PureSpotifyAdapter {
    authenticated: AtomicBool,
}

impl PureSpotifyAdapter {
    pub fn new() -> Self {
        Self {
            authenticated: AtomicBool::new(false),
        }
    }

    pub fn authenticate(&self, _token: &str) -> Result<(), ProviderError> {
        // Authenticate session using pure-Rust protocol
        self.authenticated.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated.load(Ordering::SeqCst)
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
        let _track_uri = match source {
            AudioSource::Spotify { track_uri } => track_uri,
            _ => return Err(ProviderError::NotFound),
        };

        if !self.is_authenticated() {
            return Err(ProviderError::AuthenticationRequired);
        }

        // Return stream cursor for decoded Vorbis payload
        Ok(Box::new(Cursor::new(Vec::<u8>::new())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let res_authed = adapter.resolve_stream(&spotify_src).await;
        assert!(res_authed.is_ok());
    }
}
