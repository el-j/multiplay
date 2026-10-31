//! Pure-Rust YouTube Stream Resolver and Search Adapter.
//! Implements `ProviderSearchPort` and `StreamResolverPort` using pure-Rust `rusty_ytdl`.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{
    AudioSourceStream, ProviderError, ProviderSearchPort, SharedPrecacheBuffer, StreamResolverPort,
    TrackLookupPort,
};
use async_trait::async_trait;
use rusty_ytdl::{
    search::{SearchOptions, SearchResult, SearchType, YouTube},
    Video, VideoError, VideoFormat, VideoOptions, VideoQuality, VideoSearchOptions,
};
use std::sync::Arc;
use std::time::Duration;

/// Calculates preference score for a YouTube format.
/// Prioritizes itag 140 (AAC in MP4 container) for bit-exact Symphonia compatibility.
pub fn score_youtube_format(fmt: &VideoFormat) -> i64 {
    let mut score = 0i64;
    if fmt.itag == 140 {
        // High priority: Itag 140 is standard AAC 128kbps in MP4, natively supported by symphonia
        score += 100_000;
    } else if fmt.mime_type.container.eq_ignore_ascii_case("mp4")
        || fmt.mime_type.codecs.iter().any(|c| c.contains("mp4a"))
    {
        // Secondary priority: other AAC/MP4 containers (e.g. itag 139)
        score += 50_000;
    } else if fmt.itag == 251 {
        // Opus in WebM (high bitrate ~160k)
        score += 20_000;
    } else if fmt.mime_type.container.eq_ignore_ascii_case("webm") {
        score += 10_000;
    }

    // Add bitrate bonus (1 kbps = 1 point)
    score += (fmt.bitrate / 1000) as i64;
    score
}

/// Builds VideoOptions that prefer decodable AAC/MP4 streams.
pub fn build_decodable_audio_options() -> VideoOptions {
    let comparator = Arc::new(|a: &VideoFormat, b: &VideoFormat| {
        // Sort descending: highest score comes first
        let score_a = score_youtube_format(a);
        let score_b = score_youtube_format(b);
        score_b.cmp(&score_a)
    });

    VideoOptions {
        quality: VideoQuality::Custom(VideoSearchOptions::Audio, comparator),
        filter: VideoSearchOptions::Audio,
        ..Default::default()
    }
}

/// Computes exponential backoff for retries (50ms, 150ms, 450ms).
pub fn calculate_backoff(attempt: u32) -> Duration {
    let factor = 3u64.pow(attempt.saturating_sub(1).min(3));
    Duration::from_millis(50 * factor)
}

/// Maps rusty_ytdl error to domain ProviderError.
pub fn map_ytdl_error(err: VideoError) -> ProviderError {
    match err {
        VideoError::VideoNotFound
        | VideoError::VideoIsPrivate
        | VideoError::VideoSourceNotFound => ProviderError::NotFound,
        VideoError::FormatNotFound
        | VideoError::BodyCannotParsed
        | VideoError::VideoPlayerResponseError(_) => {
            ProviderError::Unavailable(format!("YouTube extractor error: {err}"))
        }
        VideoError::Reqwest(e) => {
            if let Some(status) = e.status() {
                let code = status.as_u16();
                if code == 403 || code == 401 {
                    return ProviderError::Unavailable(format!(
                        "YouTube stream access forbidden ({code}): {e}"
                    ));
                }
            }
            ProviderError::Network(format!("YouTube network error: {e}"))
        }
        other => ProviderError::Network(format!("YouTube provider error: {other}")),
    }
}

pub struct PureYouTubeAdapter {
    searcher: YouTube,
}

impl PureYouTubeAdapter {
    pub fn new() -> Result<Self, ProviderError> {
        // rusty_ytdl's reqwest has rustls without a crypto provider; supply the pure `ring` one.
        amp_adapter_http::install_ring_provider();
        let searcher = YouTube::new().map_err(|e| {
            ProviderError::Network(format!("Failed to initialize YouTube client: {e}"))
        })?;
        Ok(Self { searcher })
    }
}

impl Default for PureYouTubeAdapter {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            searcher: YouTube::new().expect("Failed to initialize default YouTube searcher"),
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

                let source = AudioSource::YouTube { video_id: v.id };
                tracks.push(Track::new(source, metadata));
            }
        }

        Ok(tracks)
    }
}

#[async_trait]
impl TrackLookupPort for PureYouTubeAdapter {
    fn can_lookup(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::YouTube { .. })
    }

    /// Title, channel and length from the watch page, without fetching any audio.
    async fn lookup(&self, source: &AudioSource) -> Result<TrackMetadata, ProviderError> {
        let AudioSource::YouTube { video_id } = source else {
            return Err(ProviderError::NotFound);
        };
        let video = Video::new(video_id).map_err(|e| {
            ProviderError::Network(format!("Invalid YouTube video {video_id}: {e}"))
        })?;
        let info = video
            .get_basic_info()
            .await
            .map_err(|e| ProviderError::Network(format!("YouTube lookup failed: {e}")))?;
        let details = info.video_details;
        let artist = details
            .author
            .map(|a| a.name)
            .filter(|n| !n.is_empty())
            .unwrap_or(details.owner_channel_name);
        let mut metadata = TrackMetadata::new(details.title, artist);
        metadata.duration = details
            .length_seconds
            .parse::<u64>()
            .ok()
            .filter(|s| *s > 0)
            .map(Duration::from_secs);
        Ok(metadata)
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

        let video_options = build_decodable_audio_options();

        let video = Video::new_with_options(video_id, video_options).map_err(map_ytdl_error)?;

        // Attempt stream opening with backoff retry
        let mut stream_res = Err(ProviderError::NotFound);
        for attempt in 1..=3 {
            match video.stream().await {
                Ok(s) => {
                    stream_res = Ok(s);
                    break;
                }
                Err(e) => {
                    let mapped = map_ytdl_error(e);
                    // Do not retry permanent errors like NotFound or extractor broken
                    if matches!(
                        mapped,
                        ProviderError::NotFound | ProviderError::Unavailable(_)
                    ) {
                        return Err(mapped);
                    }
                    stream_res = Err(mapped);
                    if attempt < 3 {
                        tokio::time::sleep(calculate_backoff(attempt)).await;
                    }
                }
            }
        }
        let stream = stream_res?;

        let buffer = SharedPrecacheBuffer::new_blocking(4 * 1024 * 1024, Duration::from_secs(10));
        let buffer_writer = buffer.clone();

        tokio::spawn(async move {
            let mut retries = 0;
            const MAX_CHUNK_RETRIES: usize = 3;

            while !buffer_writer.is_cancelled() {
                match stream.chunk().await {
                    Ok(Some(chunk)) => {
                        retries = 0;
                        if buffer_writer.is_cancelled() {
                            break;
                        }
                        buffer_writer.push_chunk(&chunk);
                    }
                    Ok(None) => break,
                    Err(_) => {
                        retries += 1;
                        if retries > MAX_CHUNK_RETRIES {
                            break;
                        }
                        tokio::time::sleep(calculate_backoff(retries as u32)).await;
                    }
                }
            }
            buffer_writer.mark_finished();
        });

        // Await initial probing threshold (~128KB) or stream end for instant start
        let probe_threshold = 128 * 1024;
        let mut attempts = 0;
        while buffer.total_bytes_cached() < probe_threshold
            && !buffer.is_finished()
            && attempts < 100
        {
            tokio::time::sleep(Duration::from_millis(15)).await;
            attempts += 1;
        }

        if buffer.total_bytes_cached() == 0 && buffer.is_finished() {
            return Err(ProviderError::NotFound);
        }

        Ok(Box::new(buffer.into_reader()))
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

    #[tokio::test]
    async fn test_progressive_precache_buffer_stream_contract() {
        use std::io::Read;
        let buffer = SharedPrecacheBuffer::new(1024);
        let writer = buffer.clone();

        tokio::spawn(async move {
            writer.push_chunk(b"CHUNK_1_");
            tokio::time::sleep(Duration::from_millis(10)).await;
            writer.push_chunk(b"CHUNK_2");
            writer.mark_finished();
        });

        while buffer.total_bytes_cached() == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let mut reader = buffer;
        let mut out = [0u8; 8];
        let n = reader.read(&mut out).unwrap();
        assert_eq!(&out[..n], b"CHUNK_1_");

        while !reader.is_finished() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let mut out2 = [0u8; 8];
        let n2 = reader.read(&mut out2).unwrap();
        assert_eq!(&out2[..n2], b"CHUNK_2");
    }

    fn make_test_format(itag: u64, mime_str: &str, bitrate: u64) -> VideoFormat {
        let json = serde_json::json!({
            "itag": itag,
            "mimeType": mime_str,
            "bitrate": bitrate,
            "url": format!("https://rr1.googlevideo.com/videoplayback?itag={itag}"),
            "hasVideo": false,
            "hasAudio": true,
            "isLive": false,
            "isHLS": false,
            "isDashMPD": false,
        });
        serde_json::from_value(json).expect("valid VideoFormat json")
    }

    #[test]
    fn test_format_scoring_prefers_itag_140_symphonia_compatible() {
        let fmt_140 = make_test_format(140, r#"audio/mp4; codecs="mp4a.40.2""#, 128_000);
        let fmt_251 = make_test_format(251, r#"audio/webm; codecs="opus""#, 160_000);
        let fmt_139 = make_test_format(139, r#"audio/mp4; codecs="mp4a.40.2""#, 48_000);
        let fmt_249 = make_test_format(249, r#"audio/webm; codecs="opus""#, 50_000);

        let score_140 = score_youtube_format(&fmt_140);
        let score_251 = score_youtube_format(&fmt_251);
        let score_139 = score_youtube_format(&fmt_139);
        let score_249 = score_youtube_format(&fmt_249);

        // Itag 140 (AAC 128k MP4) must rank highest
        assert!(
            score_140 > score_251,
            "140 ({score_140}) should beat 251 ({score_251})"
        );
        assert!(
            score_140 > score_139,
            "140 ({score_140}) should beat 139 ({score_139})"
        );
        assert!(
            score_139 > score_251,
            "139 AAC ({score_139}) should beat 251 Opus ({score_251}) for native demuxing"
        );
        assert!(
            score_251 > score_249,
            "251 ({score_251}) should beat 249 ({score_249})"
        );

        // Test sorting
        let mut formats = [
            fmt_249.clone(),
            fmt_251.clone(),
            fmt_139.clone(),
            fmt_140.clone(),
        ];
        formats.sort_by(|a, b| {
            let sa = score_youtube_format(a);
            let sb = score_youtube_format(b);
            sb.cmp(&sa)
        });
        assert_eq!(formats[0].itag, 140);
        assert_eq!(formats[1].itag, 139);
        assert_eq!(formats[2].itag, 251);
        assert_eq!(formats[3].itag, 249);
    }

    #[test]
    fn test_error_mapping_and_backoff() {
        let not_found_err = map_ytdl_error(VideoError::VideoNotFound);
        assert_eq!(not_found_err, ProviderError::NotFound);

        let private_err = map_ytdl_error(VideoError::VideoIsPrivate);
        assert_eq!(private_err, ProviderError::NotFound);

        let extractor_err = map_ytdl_error(VideoError::FormatNotFound);
        assert!(matches!(extractor_err, ProviderError::Unavailable(_)));

        let parse_err = map_ytdl_error(VideoError::BodyCannotParsed);
        assert!(matches!(parse_err, ProviderError::Unavailable(_)));

        // Backoff progression
        assert_eq!(calculate_backoff(1), Duration::from_millis(50));
        assert_eq!(calculate_backoff(2), Duration::from_millis(150));
        assert_eq!(calculate_backoff(3), Duration::from_millis(450));
    }
}
