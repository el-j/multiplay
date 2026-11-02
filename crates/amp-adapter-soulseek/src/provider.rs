//! Soulseek as a search provider and stream resolver, through slskd. Playing a result asks
//! slskd to download it into its own folder (`<downloads>/multiplay/<id>/`) and plays the file
//! once it has arrived.

#![forbid(unsafe_code)]

use crate::slskd::{SearchResponse, SlskdClient};
use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{AudioSourceStream, ProviderError, ProviderSearchPort, StreamResolverPort};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use std::time::Duration;

/// How long a search waits for peers to answer.
const SEARCH_WAIT: Duration = Duration::from_secs(6);
/// Results kept from one search.
const MAX_RESULTS: usize = 50;
/// How long a download may take before playing gives up.
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Soulseek through slskd; does nothing until [`Self::connect`].
#[derive(Default)]
pub struct SoulseekProvider {
    client: RwLock<Option<SlskdClient>>,
    /// File sizes from searches: slskd needs the size to queue a download.
    sizes: RwLock<HashMap<(String, String), u64>>,
}

fn not_set_up() -> ProviderError {
    ProviderError::Unavailable(
        "Soulseek is not set up: switch it on in the P2P tab of Preferences".into(),
    )
}

/// The last component of a Soulseek path (`\` separated, sometimes `/`).
fn components(path: &str) -> Vec<&str> {
    path.split(['\\', '/']).filter(|c| !c.is_empty()).collect()
}

/// "02 - An Eagle in Your Mind" → "An Eagle in Your Mind".
fn strip_track_number(stem: &str) -> &str {
    let rest = stem.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() == stem.len() {
        return stem.trim();
    }
    rest.trim_start_matches([' ', '-', '.', '_']).trim()
}

/// Title, artist and album from a shared path such as
/// `Music\Artist\Album\01 - Title.flac` or `share\Artist - Album\02 - Title.mp3`.
#[must_use]
pub fn metadata_from_path(path: &str) -> TrackMetadata {
    let parts = components(path);
    let file = parts.last().copied().unwrap_or(path);
    let stem = Path::new(file)
        .file_stem()
        .map_or(file.to_string(), |s| s.to_string_lossy().into_owned());
    let mut title = strip_track_number(&stem).to_string();
    let parent = parts
        .len()
        .checked_sub(2)
        .and_then(|i| parts.get(i))
        .copied();
    let grandparent = parts
        .len()
        .checked_sub(3)
        .and_then(|i| parts.get(i))
        .copied();
    let (mut artist, mut album) = match parent.and_then(|p| p.split_once(" - ")) {
        Some((a, b)) => (a.trim().to_string(), Some(b.trim().to_string())),
        None => (
            grandparent.unwrap_or("Soulseek").to_string(),
            parent.map(str::to_string),
        ),
    };
    // "Artist - Title.mp3" in a flat folder.
    if let Some((a, t)) = title.clone().split_once(" - ") {
        if parent.is_none_or(|p| !p.contains(" - ")) && grandparent.is_none() {
            artist = a.trim().to_string();
            title = t.trim().to_string();
            album = None;
        }
    }
    let mut meta = TrackMetadata::new(title, artist);
    meta.album = album;
    meta
}

impl SoulseekProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Uses `client` from now on.
    pub fn connect(&self, client: SlskdClient) {
        if let Ok(mut c) = self.client.write() {
            *c = Some(client);
        }
    }

    /// Stops using slskd.
    pub fn disconnect(&self) {
        if let Ok(mut c) = self.client.write() {
            *c = None;
        }
    }

    /// True once connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.client.read().is_ok_and(|c| c.is_some())
    }

    fn client(&self) -> Result<SlskdClient, ProviderError> {
        self.client
            .read()
            .ok()
            .and_then(|c| c.clone())
            .ok_or_else(not_set_up)
    }

    fn tracks(&self, mut responses: Vec<SearchResponse>) -> Vec<Track> {
        // Peers that can send right away first, then short queues, then fast ones.
        responses.sort_by_key(|r| {
            (
                !r.has_free_upload_slot,
                r.queue_length,
                std::cmp::Reverse(r.upload_speed),
            )
        });
        let mut sizes = self.sizes.write().ok();
        let mut out = Vec::new();
        for r in &responses {
            for f in &r.files {
                let name = components(&f.filename)
                    .last()
                    .copied()
                    .unwrap_or_default()
                    .to_string();
                if !amp_domain::formats::is_playable_path(Path::new(&name)) {
                    continue;
                }
                let mut meta = metadata_from_path(&f.filename);
                meta.duration = f.length.map(|s| Duration::from_secs(u64::from(s)));
                meta.bitrate_kbps = f.bit_rate;
                if let Some(s) = sizes.as_mut() {
                    s.insert((r.username.clone(), f.filename.clone()), f.size);
                }
                out.push(Track::new(
                    AudioSource::Soulseek {
                        username: r.username.clone(),
                        filename: f.filename.clone(),
                    },
                    meta,
                ));
                if out.len() >= MAX_RESULTS {
                    return out;
                }
            }
        }
        out
    }
}

fn provider_error(e: crate::slskd::SlskdError) -> ProviderError {
    match e {
        crate::slskd::SlskdError::Unauthorized => ProviderError::AuthenticationRequired,
        crate::slskd::SlskdError::NotRunning(m) => ProviderError::Network(format!("slskd: {m}")),
        crate::slskd::SlskdError::Api(m) => ProviderError::Unavailable(format!("slskd: {m}")),
    }
}

#[async_trait]
impl ProviderSearchPort for SoulseekProvider {
    fn provider_id(&self) -> &'static str {
        "soulseek"
    }

    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError> {
        let client = self.client()?;
        let responses = client
            .search(query, SEARCH_WAIT)
            .await
            .map_err(provider_error)?;
        Ok(self.tracks(responses))
    }
}

#[async_trait]
impl StreamResolverPort for SoulseekProvider {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::Soulseek { .. })
    }

    fn resolve_timeout(&self) -> Option<Duration> {
        Some(DOWNLOAD_TIMEOUT + Duration::from_secs(10))
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        let AudioSource::Soulseek { username, filename } = source else {
            return Err(ProviderError::NotFound);
        };
        let client = self.client()?;
        let size = self
            .sizes
            .read()
            .ok()
            .and_then(|s| s.get(&(username.clone(), filename.clone())).copied())
            .unwrap_or(0);
        let destination = format!("multiplay/{}", uuid::Uuid::new_v4());
        client
            .download(username, filename, size, &destination)
            .await
            .map_err(provider_error)?;

        let deadline = tokio::time::Instant::now() + DOWNLOAD_TIMEOUT;
        loop {
            if let Some(t) = client
                .transfer(username, filename)
                .await
                .map_err(provider_error)?
            {
                if t.state.contains("Succeeded") {
                    break;
                }
                if t.state.contains("Completed") {
                    let why = t.exception.filter(|e| !e.is_empty()).unwrap_or(t.state);
                    return Err(ProviderError::Unavailable(format!(
                        "the Soulseek download failed: {why}"
                    )));
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(ProviderError::Timeout);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        let dir = client
            .downloads_dir()
            .await
            .map_err(provider_error)?
            .join(&destination);
        let name = components(filename)
            .last()
            .copied()
            .unwrap_or_default()
            .to_string();
        let path = if dir.join(&name).is_file() {
            dir.join(&name)
        } else {
            // slskd sanitizes names; it is the only file in its own folder.
            std::fs::read_dir(&dir)
                .ok()
                .and_then(|mut d| d.find_map(|e| e.ok().map(|e| e.path()).filter(|p| p.is_file())))
                .ok_or_else(|| {
                    ProviderError::Unavailable(format!("the download is not in {}", dir.display()))
                })?
        };
        let file =
            std::fs::File::open(&path).map_err(|e| ProviderError::Unavailable(e.to_string()))?;
        Ok(Box::new(file))
    }
}
