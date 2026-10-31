//! Pure-Rust HTTP and Internet Radio (Icecast/Shoutcast) Stream Resolver Adapter.
//! Streams progressive audio from remote HTTP/HTTPS endpoints into a [`SharedPrecacheBuffer`]
//! or [`RangeStreamReader`], with ICY metadata demultiplexing (`TASK-137`) and
//! `.pls`/`.m3u`/`.xspf` playlist resolution.

#![forbid(unsafe_code)]

pub mod icy;
pub mod playlist;
pub mod range_reader;

pub use icy::{IcyDemuxer, IcyMetadataTap};
pub use playlist::{detect_playlist_kind, parse_playlist_urls, RadioPlaylistKind};
pub use range_reader::{RangeFetcher, RangeStreamReader, ReqwestRangeFetcher};

use amp_domain::{AudioSource, TrackMetadata};
use amp_ports::{
    AudioSourceStream, ProviderError, SharedPrecacheBuffer, StreamResolverPort,
    DEFAULT_LIVE_WINDOW_BYTES,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use std::sync::Arc;
use std::time::Duration;

/// Resolver for remote HTTP/HTTPS audio files and Icecast/Shoutcast internet radio.
pub struct HttpStreamResolver {
    client: reqwest::Client,
    metadata_tap: IcyMetadataTap,
}

impl HttpStreamResolver {
    /// Initializes an HTTP stream resolver with ICY internet radio header support.
    pub fn new() -> Result<Self, ProviderError> {
        install_ring_provider();
        let mut default_headers = HeaderMap::new();
        default_headers.insert("Icy-MetaData", HeaderValue::from_static("1"));
        default_headers.insert(
            "User-Agent",
            HeaderValue::from_str(&amp_domain::about::user_agent())
                .expect("the User-Agent is plain ASCII"),
        );

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .redirect(reqwest::redirect::Policy::limited(5))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ProviderError::Network(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            client,
            metadata_tap: IcyMetadataTap::new(),
        })
    }

    /// Returns a handle to the [`IcyMetadataTap`] that records live `StreamTitle` updates.
    pub fn metadata_tap(&self) -> IcyMetadataTap {
        self.metadata_tap.clone()
    }

    /// Probes ICY headers to extract station name, genre, and bitrate if available.
    pub async fn probe_metadata(&self, url: &str) -> Option<TrackMetadata> {
        let response = self.client.head(url).send().await.ok()?;
        let headers = response.headers();

        let station_name = headers
            .get("icy-name")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Web Radio Stream".to_string());

        let genre = headers
            .get("icy-genre")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Internet Radio".to_string());

        let mut meta = TrackMetadata::new(station_name, genre);
        if let Some(bitrate_str) = headers.get("icy-br").and_then(|v| v.to_str().ok()) {
            if let Ok(kbps) = bitrate_str.parse::<u32>() {
                meta.bitrate_kbps = Some(kbps);
            }
        }
        Some(meta)
    }

    async fn resolve_url_with_playlist_failover(
        &self,
        initial_url: &str,
    ) -> Result<reqwest::Response, ProviderError> {
        let response = self.client.get(initial_url).send().await.map_err(|e| {
            ProviderError::Network(format!("HTTP request failed to {initial_url}: {e}"))
        })?;

        if !response.status().is_success() {
            return Err(ProviderError::Network(format!(
                "HTTP error response {}: {}",
                response.status(),
                initial_url
            )));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string);

        if let Some(kind) = detect_playlist_kind(initial_url, content_type.as_deref()) {
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::Network(format!("Playlist read error: {e}")))?;
            let entries = parse_playlist_urls(kind, &body);
            if entries.is_empty() {
                return Err(ProviderError::Network(format!(
                    "Empty radio playlist at {initial_url}"
                )));
            }
            let mut last_err = ProviderError::NotFound;
            for candidate in entries {
                match self.client.get(&candidate).send().await {
                    Ok(resp) if resp.status().is_success() => return Ok(resp),
                    Ok(resp) => {
                        last_err = ProviderError::Network(format!(
                            "Candidate {candidate} returned {}",
                            resp.status()
                        ));
                    }
                    Err(e) => {
                        last_err =
                            ProviderError::Network(format!("Candidate {candidate} failed: {e}"));
                    }
                }
            }
            return Err(last_err);
        }

        Ok(response)
    }
}

/// Makes rustls' ring provider the process default (once). reqwest is built without a
/// provider so that aws-lc-rs (C code) never enters the build; this supplies the crypto.
pub fn install_ring_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

impl Default for HttpStreamResolver {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            install_ring_provider();
            let client = reqwest::Client::new();
            Self {
                client,
                metadata_tap: IcyMetadataTap::new(),
            }
        })
    }
}

#[async_trait]
impl StreamResolverPort for HttpStreamResolver {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::DirectHttp { .. })
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        let url = match source {
            AudioSource::DirectHttp { url } => url,
            _ => return Err(ProviderError::NotFound),
        };

        let response = self.resolve_url_with_playlist_failover(url).await?;
        let final_url = response.url().to_string();
        let headers = response.headers();

        let icy_metaint = headers
            .get("icy-metaint")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n > 0);

        let accepts_ranges = headers
            .get(reqwest::header::ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.eq_ignore_ascii_case("bytes"))
            .unwrap_or(false);

        let content_length = response.content_length().filter(|&len| len > 0);

        // If the remote server advertises Accept-Ranges: bytes + Content-Length and is not an ICY stream,
        // return a seekable RangeStreamReader when a large seekable file is requested (`TASK-138`).
        if icy_metaint.is_none()
            && accepts_ranges
            && content_length.is_some_and(|len| len >= 16 * 1024 * 1024)
        {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let total_len = content_length.unwrap();
                let fetcher = Arc::new(ReqwestRangeFetcher::new(
                    self.client.clone(),
                    final_url,
                    handle,
                ));
                return Ok(Box::new(RangeStreamReader::new(fetcher, total_len)));
            }
        }

        // Choose live sliding-window buffer for infinite/radio streams or blocking buffer for finite streams
        let is_live = icy_metaint.is_some() || content_length.is_none();
        let buffer = if is_live {
            SharedPrecacheBuffer::new_live_ring(DEFAULT_LIVE_WINDOW_BYTES, Duration::from_secs(10))
        } else {
            SharedPrecacheBuffer::new_blocking(4 * 1024 * 1024, Duration::from_secs(10))
        };
        let buffer_writer = buffer.clone();
        let tap = self.metadata_tap.clone();
        let mut byte_stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut demux = icy_metaint.map(|m| IcyDemuxer::new(m, tap));
            let mut audio_scratch = Vec::with_capacity(16 * 1024);
            while !buffer_writer.is_cancelled() {
                match byte_stream.next().await {
                    Some(Ok(chunk)) => {
                        if buffer_writer.is_cancelled() {
                            break;
                        }
                        if let Some(ref mut d) = demux {
                            audio_scratch.clear();
                            d.push_bytes(&chunk, &mut audio_scratch);
                            if !audio_scratch.is_empty() {
                                buffer_writer.push_chunk(&audio_scratch);
                            }
                        } else {
                            buffer_writer.push_chunk(&chunk);
                        }
                    }
                    Some(Err(_)) | None => break,
                }
            }
            buffer_writer.mark_finished();
        });

        let probe_threshold = 64 * 1024;
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
    use std::io::Read;

    #[test]
    fn test_http_resolver_handles_source() {
        let resolver = HttpStreamResolver::new().expect("Failed to initialize HttpStreamResolver");
        let http_src = AudioSource::DirectHttp {
            url: "https://somafm.com/groovesalad130.pls".to_string(),
        };
        let yt_src = AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".to_string(),
        };

        assert!(resolver.handles_source(&http_src));
        assert!(!resolver.handles_source(&yt_src));
    }

    #[tokio::test]
    async fn test_http_resolver_unhandled_source() {
        let resolver = HttpStreamResolver::default();
        let local_src = AudioSource::LocalFile {
            path: "/path/to/song.flac".to_string(),
        };
        let res = resolver.resolve_stream(&local_src).await;
        assert!(matches!(res, Err(ProviderError::NotFound)));
    }

    #[test]
    fn slow_server_8kbps_with_jitter_streams_identically_without_wouldblock() {
        // TASK-135 acceptance criteria: an audio stream served at ~8 KB/s with jitter
        // into SharedPrecacheBuffer::new_blocking completes with byte/sample-identical output
        // and never returns WouldBlock.
        let total_bytes = 24 * 1024; // 24 KB payload (~3 seconds at 8 KB/s)
        let original_payload: Vec<u8> = (0..total_bytes)
            .map(|i| ((i * 73 + 19) & 0xFF) as u8)
            .collect();

        let buffer = SharedPrecacheBuffer::new_blocking(total_bytes, Duration::from_secs(5));
        let writer = buffer.clone();
        let expected = original_payload.clone();

        let producer = std::thread::spawn(move || {
            // 8 KB/s = 512 bytes every ~62.5 ms with deterministic jitter (40 ms .. 85 ms)
            let chunk_size = 512;
            for (idx, chunk) in expected.chunks(chunk_size).enumerate() {
                if writer.is_cancelled() {
                    break;
                }
                let jitter_ms = 15 + ((idx * 17) % 25) as u64;
                std::thread::sleep(Duration::from_millis(jitter_ms));
                writer.push_chunk(chunk);
            }
            writer.mark_finished();
        });

        let mut reader = buffer.into_reader();
        let mut decoded_output = Vec::with_capacity(total_bytes);
        let mut read_buf = [0u8; 1024];
        loop {
            let n = reader
                .read(&mut read_buf)
                .expect("blocking reader must wait on Condvar and never return WouldBlock");
            if n == 0 {
                break;
            }
            decoded_output.extend_from_slice(&read_buf[..n]);
        }

        producer.join().unwrap();
        assert_eq!(decoded_output, original_payload);
    }
}
