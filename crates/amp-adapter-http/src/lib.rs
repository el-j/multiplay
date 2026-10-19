//! Pure-Rust HTTP and Internet Radio (Icecast/Shoutcast) Stream Resolver Adapter.
//! Streams progressive audio from remote HTTP/HTTPS endpoints into a `SharedPrecacheBuffer`.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, TrackMetadata};
use amp_ports::{AudioSourceStream, ProviderError, SharedPrecacheBuffer, StreamResolverPort};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

/// Resolver for remote HTTP/HTTPS audio files and Icecast/Shoutcast internet radio.
pub struct HttpStreamResolver {
    client: reqwest::Client,
}

impl HttpStreamResolver {
    /// Initializes an HTTP stream resolver with ICY internet radio header support.
    pub fn new() -> Result<Self, ProviderError> {
        install_ring_provider();
        let mut default_headers = HeaderMap::new();
        default_headers.insert("Icy-MetaData", HeaderValue::from_static("1"));
        default_headers.insert(
            "User-Agent",
            HeaderValue::from_static("Multiplay/1.0 (Pure-Rust Audiophile Player)"),
        );

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ProviderError::Network(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self { client })
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
}

/// Makes rustls' ring provider the process default (once). reqwest is built without a
/// provider so that aws-lc-rs (C code) never enters the build; this supplies the crypto.
pub fn install_ring_provider() {
    // Fails only when a provider is already installed, which is fine.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

impl Default for HttpStreamResolver {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            install_ring_provider();
            let client = reqwest::Client::new();
            Self { client }
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

        let response =
            self.client.get(url).send().await.map_err(|e| {
                ProviderError::Network(format!("HTTP request failed to {url}: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(ProviderError::Network(format!(
                "HTTP error response {}: {}",
                response.status(),
                url
            )));
        }

        let mut byte_stream = response.bytes_stream();
        let buffer = SharedPrecacheBuffer::new(4 * 1024 * 1024);
        let buffer_writer = buffer.clone();

        tokio::spawn(async move {
            while let Some(chunk_res) = byte_stream.next().await {
                match chunk_res {
                    Ok(chunk) => {
                        buffer_writer.push_chunk(&chunk);
                    }
                    Err(e) => {
                        eprintln!("HTTP stream chunk error: {e}");
                        break;
                    }
                }
            }
            buffer_writer.mark_finished();
        });

        // Await initial probing threshold (~128KB) or early completion
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

        Ok(Box::new(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
