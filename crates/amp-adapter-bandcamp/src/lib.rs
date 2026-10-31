//! Pure-Rust Bandcamp Search, Page Scraper and Stream Resolver Adapter (`TASK-159`, `SRC-11`).
//!
//! Searches Bandcamp via its public search API, extracts tracks and `mp3-128` streams
//! from `data-tralbum` JSON, and resolves audio streams for playback.

#![forbid(unsafe_code)]

use amp_adapter_http::{install_ring_provider, HttpStreamResolver};
use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{
    AudioSourceStream, ProviderError, ProviderSearchPort, StreamResolverPort, TrackLookupPort,
};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

/// Bandcamp's search endpoint (the one bandcamp.com's own search box uses).
const SEARCH_ENDPOINT: &str = "https://bandcamp.com/api/bcsearch_public_api/1/autocomplete_elastic";

/// Search response: results, or `{"error": true, "error_message": ...}`.
#[derive(Debug, Deserialize)]
struct SearchResponse {
    auto: Option<AutoSection>,
    #[serde(default)]
    error: bool,
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AutoSection {
    #[serde(default)]
    results: Vec<SearchResult>,
}

/// One hit: `type` is `a` (album), `t` (track) or `b` (artist/label page).
#[derive(Debug, Deserialize)]
struct SearchResult {
    #[serde(rename = "type")]
    item_type: String,
    name: Option<String>,
    band_name: Option<String>,
    item_url_path: Option<String>,
    art_id: Option<u64>,
}

/// Bandcamp tralbum embedded JSON data structure.
#[derive(Debug, Deserialize)]
pub struct TralbumData {
    pub current: Option<TralbumCurrent>,
    pub trackinfo: Option<Vec<TralbumTrack>>,
    pub artist: Option<String>,
    pub art_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TralbumCurrent {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TralbumTrack {
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub file: Option<TralbumFile>,
    #[allow(dead_code)]
    pub title_link: Option<String>,
    #[allow(dead_code)]
    pub track_num: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct TralbumFile {
    #[serde(rename = "mp3-128")]
    pub mp3_128: Option<String>,
}

pub struct PureBandcampAdapter {
    client: reqwest::Client,
    http_resolver: Arc<HttpStreamResolver>,
}

impl PureBandcampAdapter {
    pub fn new() -> Result<Self, ProviderError> {
        install_ring_provider();
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            "User-Agent",
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) multiplay/1.0"),
        );

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .connect_timeout(Duration::from_secs(8))
            .build()
            .map_err(|e| ProviderError::Network(format!("Failed to build HTTP client: {e}")))?;

        let http_resolver = Arc::new(HttpStreamResolver::new()?);

        Ok(Self {
            client,
            http_resolver,
        })
    }
}

impl Default for PureBandcampAdapter {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Bandcamp adapter")
    }
}

/// Parses HTML content to extract embedded `data-tralbum` JSON object (`TASK-159`).
pub fn extract_tralbum_json(html: &str) -> Option<TralbumData> {
    let marker = "data-tralbum=\"";
    let start_idx = html.find(marker)? + marker.len();
    let end_idx = html[start_idx..].find('"')? + start_idx;
    let raw_encoded = &html[start_idx..end_idx];

    let unescaped = raw_encoded
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">");

    serde_json::from_str(&unescaped).ok()
}

/// Parses a Bandcamp search response into playable results: albums (releases) and tracks.
/// Artist pages are skipped. An error response is returned as an error, never as "no results".
pub fn parse_search_response(json: &str) -> Result<Vec<Track>, ProviderError> {
    let resp: SearchResponse = serde_json::from_str(json).map_err(|e| {
        ProviderError::Unavailable(format!("Unexpected Bandcamp search response: {e}"))
    })?;
    if resp.error {
        return Err(ProviderError::Unavailable(format!(
            "Bandcamp search error: {}",
            resp.error_message.as_deref().unwrap_or("unknown")
        )));
    }
    let results = resp.auto.map(|a| a.results).unwrap_or_default();
    Ok(results
        .into_iter()
        .filter(|r| r.item_type == "a" || r.item_type == "t")
        .filter_map(|r| {
            let url = r.item_url_path?;
            let title = r.name.unwrap_or_else(|| "Unknown Title".into());
            let mut meta = TrackMetadata::new(
                title.clone(),
                r.band_name.unwrap_or_else(|| "Unknown Artist".into()),
            );
            if r.item_type == "a" {
                meta.album = Some(title);
            }
            meta.cover_art_uri = r
                .art_id
                .map(|art| format!("https://f4.bcbits.com/img/a{art}_2.jpg"));
            Some(Track::new(AudioSource::Bandcamp { stream_url: url }, meta))
        })
        .collect())
}

/// The `mp3-128` stream of the first track that has one. Pages of artists who turned streaming
/// off list their tracks without audio files.
pub fn stream_url(tralbum: TralbumData) -> Result<String, ProviderError> {
    tralbum
        .trackinfo
        .unwrap_or_default()
        .into_iter()
        .find_map(|t| t.file.and_then(|f| f.mp3_128))
        .ok_or_else(|| {
            ProviderError::Unavailable(
                "This Bandcamp release cannot be streamed (the artist turned streaming off)".into(),
            )
        })
}

#[async_trait]
impl ProviderSearchPort for PureBandcampAdapter {
    fn provider_id(&self) -> &'static str {
        "bandcamp"
    }

    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError> {
        let body = serde_json::json!({
            "search_text": query,
            "search_filter": "",
            "full_page": false,
            "fan_id": null,
        });
        let resp = self
            .client
            .post(SEARCH_ENDPOINT)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| ProviderError::Network(format!("Bandcamp search failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(ProviderError::Network(format!(
                "Bandcamp search HTTP {}",
                resp.status()
            )));
        }

        let body = resp.text().await.map_err(|e| {
            ProviderError::Network(format!("Failed to read Bandcamp response: {e}"))
        })?;

        parse_search_response(&body)
    }
}

#[async_trait]
impl TrackLookupPort for PureBandcampAdapter {
    fn can_lookup(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::Bandcamp { .. })
    }

    async fn lookup(&self, source: &AudioSource) -> Result<TrackMetadata, ProviderError> {
        let AudioSource::Bandcamp { stream_url } = source else {
            return Err(ProviderError::NotFound);
        };

        let resp = self
            .client
            .get(stream_url)
            .send()
            .await
            .map_err(|e| ProviderError::Network(format!("Bandcamp lookup failed: {e}")))?;

        let html = resp
            .text()
            .await
            .map_err(|e| ProviderError::Network(format!("Failed to read page: {e}")))?;

        let tralbum = extract_tralbum_json(&html).ok_or_else(|| {
            ProviderError::Network("Missing data-tralbum on Bandcamp page".into())
        })?;

        let artist = tralbum
            .artist
            .or_else(|| tralbum.current.as_ref().and_then(|c| c.artist.clone()))
            .unwrap_or_else(|| "Unknown Artist".into());

        let title = tralbum
            .current
            .as_ref()
            .and_then(|c| c.title.clone())
            .unwrap_or_else(|| "Unknown Track".into());

        let mut meta = TrackMetadata::new(title, artist);
        if let Some(c) = tralbum.current.as_ref() {
            meta.album = c.album_title.clone();
        }
        if let Some(art) = tralbum.art_id {
            meta.cover_art_uri = Some(format!("https://f4.bcbits.com/img/a{art}_2.jpg"));
        }
        if let Some(tracks) = tralbum.trackinfo.as_ref().and_then(|t| t.first()) {
            if let Some(d) = tracks.duration {
                meta.duration = Some(Duration::from_secs_f64(d));
            }
        }

        Ok(meta)
    }
}

#[async_trait]
impl StreamResolverPort for PureBandcampAdapter {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::Bandcamp { .. })
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        let AudioSource::Bandcamp { stream_url } = source else {
            return Err(ProviderError::NotFound);
        };

        // If direct stream URL, delegate directly
        if stream_url.contains("bcbits.com") || stream_url.ends_with(".mp3") {
            let direct_src = AudioSource::DirectHttp {
                url: stream_url.clone(),
            };
            return self.http_resolver.resolve_stream(&direct_src).await;
        }

        // Scrape page for mp3-128 stream URL
        let resp =
            self.client.get(stream_url).send().await.map_err(|e| {
                ProviderError::Network(format!("Bandcamp stream fetch failed: {e}"))
            })?;

        let html = resp
            .text()
            .await
            .map_err(|e| ProviderError::Network(format!("Failed to read Bandcamp page: {e}")))?;

        let tralbum = extract_tralbum_json(&html)
            .ok_or_else(|| ProviderError::Network("No tralbum audio data found on page".into()))?;

        let direct_src = AudioSource::DirectHttp {
            url: crate::stream_url(tralbum)?,
        };
        self.http_resolver.resolve_stream(&direct_src).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tralbum_json_fixture() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Song Page</title></head>
            <body>
            <script type="text/javascript" data-tralbum="{&quot;current&quot;:{&quot;title&quot;:&quot;Selected Track&quot;,&quot;artist&quot;:&quot;Artist Name&quot;,&quot;album_title&quot;:&quot;Album Name&quot;},&quot;trackinfo&quot;:[{&quot;title&quot;:&quot;Selected Track&quot;,&quot;duration&quot;:245.5,&quot;file&quot;:{&quot;mp3-128&quot;:&quot;https://m4.bcbits.com/stream/stream.mp3&quot;}}],&quot;art_id&quot;:998877}"></script>
            </body>
            </html>
        "#;

        let tralbum = extract_tralbum_json(html).expect("Failed to extract tralbum");
        assert_eq!(
            tralbum.current.as_ref().and_then(|c| c.title.as_deref()),
            Some("Selected Track")
        );
        let tracks = tralbum.trackinfo.expect("Missing trackinfo");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].duration, Some(245.5));
        assert_eq!(
            tracks[0].file.as_ref().and_then(|f| f.mp3_128.as_deref()),
            Some("https://m4.bcbits.com/stream/stream.mp3")
        );
    }
}
