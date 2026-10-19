#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackId};
use async_trait::async_trait;
use std::io::{Read, Seek};
use std::time::Duration;
use thiserror::Error;

/// Stream handle providing synchronous Read and Seek required by audio decoders.
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

/// Driven port for resolving audio sources into playable byte streams.
#[async_trait]
pub trait StreamResolverPort: Send + Sync {
    fn handles_source(&self, source: &AudioSource) -> bool;
    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError>;
}

/// Driven port for searching external online or network audio providers.
#[async_trait]
pub trait ProviderSearchPort: Send + Sync {
    fn provider_id(&self) -> &'static str;
    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError>;
}

/// Driven port for a decoded audio stream: interleaved stereo `f32` frames at the stream's own
/// sample rate. The DJ engine loads deck tracks through it, so the core never names a decoder.
pub trait PcmStreamPort: Send {
    /// Native sample rate of the decoded audio in Hz.
    fn sample_rate(&self) -> u32;
    /// Fills `out` with up to `out.len()` interleaved stereo samples `[L, R, ...]` and returns how
    /// many were written; `Ok(0)` means the end of the stream.
    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, String>;
}

/// Driven port for cached DJ track analysis, keyed by `amp_domain::analysis_key`.
pub trait AnalysisCachePort: Send + Sync {
    /// The stored analysis for `key`, if any (of any version; callers check it).
    fn load(&self, key: &str) -> Option<amp_domain::TrackAnalysis>;
    /// Stores `analysis` under `key`, replacing an older one.
    fn store(&self, key: &str, analysis: &amp_domain::TrackAnalysis) -> Result<(), String>;
}

/// Driven port for hardware audio output device sinks.
pub trait AudioSinkPort: Send + Sync {
    fn configure(&mut self, sample_rate: u32, channels: u16) -> Result<(), String>;
    fn start(&mut self) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn push_interleaved_samples(&mut self, samples: &[f32]) -> usize;
    fn get_buffer_latency(&self) -> Duration;
}

/// Driven port for embedded library database and index storage.
#[async_trait]
pub trait LibraryStoragePort: Send + Sync {
    async fn store_track(&self, track: &Track) -> Result<(), String>;
    async fn fetch_all(&self) -> Result<Vec<Track>, String>;
    async fn remove_track(&self, id: &TrackId) -> Result<(), String>;
    async fn search_cached(&self, text_query: &str) -> Result<Vec<Track>, String>;
}

/// Driven port for secure credential and secret token storage.
#[async_trait]
pub trait CredentialVaultPort: Send + Sync {
    async fn store_secret(&self, key: &str, secret: &str) -> Result<(), String>;
    async fn get_secret(&self, key: &str) -> Result<Option<String>, String>;
    async fn delete_secret(&self, key: &str) -> Result<(), String>;
}
