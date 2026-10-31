#![forbid(unsafe_code)]

use crate::driven::ProviderError;
use crate::error::{DownloadError, PlaylistError};
use amp_domain::source_badge::{SourceFilterMask, SourceKind};
use amp_domain::{PlaybackState, Track, TrackId};
use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;

/// Typed errors returned by `PlayerControlPort`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
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

/// Driving port for controlling playback actions from user interfaces (TUI, GUI, CLI, Remote).
#[async_trait]
pub trait PlayerControlPort: Send + Sync {
    async fn play_track(&self, track: Track) -> Result<(), PlayerControlError>;
    async fn toggle_pause(&self) -> Result<(), PlayerControlError>;
    async fn stop(&self) -> Result<(), PlayerControlError>;
    async fn seek_relative(
        &self,
        offset: Duration,
        forward: bool,
    ) -> Result<(), PlayerControlError>;
    async fn seek_absolute(&self, timestamp: Duration) -> Result<(), PlayerControlError>;
    fn current_state(&self) -> PlaybackState;
    fn current_position(&self) -> Duration;
}

/// Driving port for managing active playlist items and queue ordering.
#[async_trait]
pub trait PlaylistManagementPort: Send + Sync {
    async fn add_to_playlist(&self, track: Track) -> Result<usize, PlaylistError>;
    async fn remove_from_playlist(&self, track_id: TrackId) -> Result<(), PlaylistError>;
    async fn clear_playlist(&self) -> Result<(), PlaylistError>;
    async fn move_track(&self, from_idx: usize, to_idx: usize) -> Result<(), PlaylistError>;
    async fn get_playlist(&self) -> Vec<Track>;
}

/// Driving port for executing simple text queries across all sources.
#[async_trait]
pub trait UniversalSearchPort: Send + Sync {
    async fn execute_search(&self, query: &str) -> Vec<Track>;
}

/// Outcome of a search query executed against a single provider (`TASK-154`).
#[derive(Debug, Clone)]
pub struct ProviderOutcome {
    pub kind: SourceKind,
    pub result: Result<Vec<Track>, ProviderError>,
    pub latency: Duration,
}

impl ProviderOutcome {
    /// Returns the UI chip icon and status label for rendering search status chips.
    pub fn status_chip(&self) -> (&'static str, &'static str) {
        match &self.result {
            Ok(tracks) if tracks.is_empty() => ("∅", "No results"),
            Ok(_) => ("✓", "Results ready"),
            Err(ProviderError::AuthenticationRequired) => ("🔒", "Login required"),
            Err(ProviderError::RateLimited) => ("⚠", "Rate limited"),
            Err(ProviderError::Timeout) => ("⏳", "Timed out"),
            Err(ProviderError::Network(_)) => ("⚠", "Network error"),
            Err(ProviderError::Unavailable(_)) => ("⚠", "Unavailable"),
            Err(ProviderError::NotFound) => ("∅", "Not found"),
        }
    }
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
    async fn search_outcomes(&self, options: SearchQueryOptions) -> Vec<ProviderOutcome>;
}

/// Driving port for background track downloading and local caching.
#[async_trait]
pub trait TrackDownloadPort: Send + Sync {
    async fn queue_download(
        &self,
        track: Track,
        target_directory: String,
    ) -> Result<String, DownloadError>;
    async fn cancel_download(&self, track_id: TrackId) -> Result<(), DownloadError>;
}
