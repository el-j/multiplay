//! Pure-Rust YouTube Stream Resolver and Search Adapter.
//! Implements `ProviderSearchPort` and `StreamResolverPort` using pure-Rust `rusty_ytdl`.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{AudioSourceStream, ProviderError, ProviderSearchPort, StreamResolverPort};
use async_trait::async_trait;
use rusty_ytdl::{
    search::{SearchOptions, SearchResult, SearchType, YouTube},
    Video, VideoOptions, VideoQuality, VideoSearchOptions,
};
use std::io::Cursor;
use std::time::Duration;

pub struct PureYouTubeAdapter {
    searcher: YouTube,
}

impl PureYouTubeAdapter {
    pub fn new() -> Result<Self, ProviderError> {
        let searcher = YouTube::new()
            .map_err(|e| ProviderError::Network(format!("Failed to initialize YouTube client: {e}")))?;
        Ok(Self { searcher })
    }
}

impl Default for PureYouTubeAdapter {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            Self {
                searcher: YouTube::new().expect("Failed to initialize default YouTube searcher"),
            }
        })
    }
}

#[async_trait]
impl ProviderSearchPort for PureYouTubeAdapter {
    fn provider_id(&self) -> &'static str {
        "youtube"
    }

    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError> {
        let options = SearchOptions {
            limit: 25,
            search_type: SearchType::Video,
            safe_search: false,
        };

        let results = self
            .searcher
            .search(query, Some(&options))
            .await
            .map_err(|e| ProviderError::Network(format!("YouTube search failed: {e}")))?;

        let mut tracks = Vec::new();
        for item in results {
            if let SearchResult::Video(v) = item {
                let mut metadata = TrackMetadata::new(v.title, v.channel.name);
                if v.duration > 0 {
                    metadata.duration = Some(Duration::from_secs(v.duration / 1000));
                }
                if let Some(thumb) = v.thumbnails.last() {
                    metadata.cover_art_uri = Some(thumb.url.clone());
                }

                let source = AudioSource::YouTube {
                    video_id: v.id,
                };
                tracks.push(Track::new(source, metadata));
            }
        }

        Ok(tracks)
    }
}

#[async_trait]
impl StreamResolverPort for PureYouTubeAdapter {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::YouTube { .. })
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        let video_id = match source {
            AudioSource::YouTube { video_id } => video_id,
            _ => return Err(ProviderError::NotFound),
        };

        let video_options = VideoOptions {
            quality: VideoQuality::HighestAudio,
            filter: VideoSearchOptions::Audio,
            ..Default::default()
        };

        let video = Video::new_with_options(video_id, video_options)
            .map_err(|e| ProviderError::Network(format!("Invalid YouTube video {video_id}: {e}")))?;

        let stream = video
            .stream()
            .await
            .map_err(|e| ProviderError::Network(format!("Failed to stream YouTube audio: {e}")))?;

        let mut bytes = Vec::new();
        while let Some(chunk) = stream
            .chunk()
            .await
            .map_err(|e| ProviderError::Network(format!("Stream error reading chunk: {e}")))?
        {
            bytes.extend_from_slice(&chunk);
        }

        if bytes.is_empty() {
            return Err(ProviderError::NotFound);
        }

        let cursor: Cursor<Vec<u8>> = Cursor::new(bytes);
        Ok(Box::new(cursor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation_and_handles_source() {
        let adapter = PureYouTubeAdapter::new();
        assert!(adapter.is_ok());
        let adapter = adapter.unwrap();

        let yt_source = AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".to_string(),
        };
        let local_source = AudioSource::LocalFile {
            path: "/path/to/song.flac".to_string(),
        };

        assert!(adapter.handles_source(&yt_source));
        assert!(!adapter.handles_source(&local_source));
        assert_eq!(adapter.provider_id(), "youtube");
    }

    #[tokio::test]
    async fn test_unhandled_source_error() {
        let adapter = PureYouTubeAdapter::new().unwrap();
        let local_source = AudioSource::LocalFile {
            path: "/path/to/song.flac".to_string(),
        };
        let result = adapter.resolve_stream(&local_source).await;
        assert!(matches!(result, Err(ProviderError::NotFound)));
    }
}
