# Domain Models and Port Specifications

This document defines the pure domain business rules, domain entities, value objects, and decoupled port traits for driving (inbound) and driven (outbound) interactions.

---

## 1. Domain Entities & Value Objects (`amp-domain`)

```rust
// crates/amp-domain/src/lib.rs
use std::time::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub Uuid);

impl TrackId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioSource {
    LocalFile { path: String },
    NetworkSmb { share_url: String },
    DirectHttp { url: String },
    YouTube { video_id: String },
    Spotify { track_uri: String },
    Bandcamp { stream_url: String },
    SoundCloud { track_url: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub duration: Option<Duration>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub bit_depth: Option<u16>,
    pub bitrate_kbps: Option<u32>,
    pub cover_art_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub source: AudioSource,
    pub metadata: TrackMetadata,
    pub date_added_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Buffering { progress_percent: u8 },
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct AudioFrameChunk {
    pub samples: Vec<f32>, // Interleaved normalized audio (-1.0 to 1.0)
    pub channels: u16,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct BufferConfiguration {
    pub precache_capacity_bytes: usize, // e.g. 64 MB
    pub ring_buffer_samples: usize,     // e.g. 65,536 frames
    pub high_watermark_ratio: f32,       // e.g. 0.95
    pub low_watermark_ratio: f32,        // e.g. 0.15
}

impl Default for BufferConfiguration {
    fn default() -> Self {
        Self {
            precache_capacity_bytes: 64 * 1024 * 1024, // 64 MB
            ring_buffer_samples: 65536,
            high_watermark_ratio: 0.90,
            low_watermark_ratio: 0.20,
        }
    }
}
```

---

## 2. Inbound / Driving Ports (`amp-ports/src/driving.rs`)

These ports define how frontends (TUI, GUI, CLI) control the domain.

```rust
// crates/amp-ports/src/driving.rs
use std::time::Duration;
use async_trait::async_trait;
use amp_domain::{AudioSource, PlaybackState, Track, TrackId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayerControlError {
    #[error("Engine unavailable or shut down")]
    EngineTerminated,
    #[error("Source stream could not be resolved: {0}")]
    UnresolvableSource(String),
    #[error("Audio decoder failed to parse stream: {0}")]
    DecodingFailure(String),
    #[error("Hardware audio output device failed: {0}")]
    DeviceError(String),
    #[error("Seek out of bounds")]
    SeekOutOfBounds,
}

#[async_trait]
pub trait PlayerControlPort: Send + Sync {
    async fn play_track(&self, track: Track) -> Result<(), PlayerControlError>;
    async fn toggle_pause(&self) -> Result<(), PlayerControlError>;
    async fn stop(&self) -> Result<(), PlayerControlError>;
    async fn seek_relative(&self, offset: Duration, forward: bool) -> Result<(), PlayerControlError>;
    async fn seek_absolute(&self, timestamp: Duration) -> Result<(), PlayerControlError>;
    fn current_state(&self) -> PlaybackState;
    fn current_position(&self) -> Duration;
}

#[async_trait]
pub trait PlaylistManagementPort: Send + Sync {
    async fn add_to_playlist(&self, track: Track) -> Result<usize, String>;
    async fn remove_from_playlist(&self, track_id: TrackId) -> Result<(), String>;
    async fn clear_playlist(&self) -> Result<(), String>;
    async fn move_track(&self, from_idx: usize, to_idx: usize) -> Result<(), String>;
    async fn get_playlist(&self) -> Vec<Track>;
}

#[async_trait]
pub trait UniversalSearchPort: Send + Sync {
    async fn execute_search(&self, query: &str) -> Vec<Track>;
}

#[async_trait]
pub trait TrackDownloadPort: Send + Sync {
    async fn queue_download(&self, track: Track, target_directory: String) -> Result<String, String>;
    async fn cancel_download(&self, track_id: TrackId) -> Result<(), String>;
}
```

---

## 3. Outbound / Driven Ports (`amp-ports/src/driven.rs`)

These ports define interfaces required by the core from external hardware, decoders, and network providers.

```rust
// crates/amp-ports/src/driven.rs
use std::io::{Read, Seek};
use async_trait::async_trait;
use amp_domain::{AudioSource, Track, TrackMetadata};
use thiserror::Error;

pub trait AudioSourceStream: Read + Seek + Send + Sync {}
impl<T: Read + Seek + Send + Sync> AudioSourceStream for T {}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Network connection failed: {0}")]
    Network(String),
    #[error("Provider rate limited or blocked")]
    RateLimited,
    #[error("Authentication credentials missing or rejected")]
    AuthenticationRequired,
    #[error("Track not found or unavailable in this region")]
    NotFound,
}

#[async_trait]
pub trait StreamResolverPort: Send + Sync {
    fn handles_source(&self, source: &AudioSource) -> bool;
    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError>;
}

#[async_trait]
pub trait ProviderSearchPort: Send + Sync {
    fn provider_id(&self) -> &'static str;
    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError>;
}

pub trait AudioSinkPort: Send + Sync {
    fn configure(&mut self, sample_rate: u32, channels: u16) -> Result<(), String>;
    fn start(&mut self) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn push_interleaved_samples(&mut self, samples: &[f32]) -> usize;
    fn get_buffer_latency(&self) -> std::time::Duration;
}

#[async_trait]
pub trait LibraryStoragePort: Send + Sync {
    async fn store_track(&self, track: &Track) -> Result<(), String>;
    async fn fetch_all(&self) -> Result<Vec<Track>, String>;
    async fn remove_track(&self, id: &TrackId) -> Result<(), String>;
    async fn search_cached(&self, text_query: &str) -> Result<Vec<Track>, String>;
}
```