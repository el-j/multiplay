#![forbid(unsafe_code)]

use amp_domain::source_badge::SourceFilterMask;
use amp_domain::{PlaybackState, Track, TrackId};
use async_trait::async_trait;
use std::time::Duration;
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

/// Driving port for controlling playback actions from user interfaces (TUI, GUI, CLI).
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

/// Driving port for managing active playlist items and queue ordering.
#[async_trait]
pub trait PlaylistManagementPort: Send + Sync {
    async fn add_to_playlist(&self, track: Track) -> Result<usize, String>;
    async fn remove_from_playlist(&self, track_id: TrackId) -> Result<(), String>;
    async fn clear_playlist(&self) -> Result<(), String>;
    async fn move_track(&self, from_idx: usize, to_idx: usize) -> Result<(), String>;
    async fn get_playlist(&self) -> Vec<Track>;
}

/// Driving port for executing simple text queries across all sources.
#[async_trait]
pub trait UniversalSearchPort: Send + Sync {
    async fn execute_search(&self, query: &str) -> Vec<Track>;
}

/// Options for fine-grained multi-source searches.
#[derive(Debug, Clone)]
pub struct SearchQueryOptions {
    pub text: String,
    pub filter_mask: SourceFilterMask,
    pub timeout_millis: u64,
}

/// Driving port for executing searches with source filters and timeouts.
#[async_trait]
pub trait FilteredSearchPort: Send + Sync {
    async fn search(&self, options: SearchQueryOptions) -> Vec<Track>;
}

/// Driving port for background track downloading and local caching.
#[async_trait]
pub trait TrackDownloadPort: Send + Sync {
    async fn queue_download(&self, track: Track, target_directory: String) -> Result<String, String>;
    async fn cancel_download(&self, track_id: TrackId) -> Result<(), String>;
}
