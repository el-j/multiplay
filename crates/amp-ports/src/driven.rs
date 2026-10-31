#![forbid(unsafe_code)]

use crate::error::{AudioSinkError, DecodeError, StorageError, VaultError};
use amp_domain::{AudioSource, Track, TrackId, TrackMetadata};
use async_trait::async_trait;
use std::io::{Read, Seek};
use std::time::Duration;
use thiserror::Error;

/// Stream handle providing synchronous Read and Seek required by audio decoders.
pub trait AudioSourceStream: Read + Seek + Send + Sync {}
impl<T: Read + Seek + Send + Sync> AudioSourceStream for T {}

/// Typed errors returned by external stream resolvers and search providers.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderError {
    #[error("Network connection failed: {0}")]
    Network(String),
    #[error("Provider rate limited or blocked")]
    RateLimited,
    #[error("Authentication credentials missing or rejected")]
    AuthenticationRequired,
    #[error("Track not found or unavailable in this region")]
    NotFound,
    #[error("Provider query timed out")]
    Timeout,
    #[error("Provider unavailable or extractor broken: {0}")]
    Unavailable(String),
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

/// Driven port for looking up a source's title, artist and duration without streaming it, so a
/// host can preview a remote's request before accepting it (TASK-216).
#[async_trait]
pub trait TrackLookupPort: Send + Sync {
    fn can_lookup(&self, source: &AudioSource) -> bool;
    async fn lookup(&self, source: &AudioSource) -> Result<TrackMetadata, ProviderError>;
}

/// Driven port for searching external online or network audio providers.
#[async_trait]
pub trait ProviderSearchPort: Send + Sync {
    fn provider_id(&self) -> &'static str;
    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError>;
}

/// Driven port for a decoded audio stream: interleaved stereo `f32` frames at the stream's own
/// sample rate, plus optional seek and duration reporting so the core engine never names a
/// concrete decoder adapter.
pub trait PcmStreamPort: Send {
    /// Native sample rate of the decoded audio in Hz.
    fn sample_rate(&self) -> u32;
    /// Fills `out` with up to `out.len()` interleaved stereo samples `[L, R, ...]` and returns how
    /// many were written; `Ok(0)` means the end of the stream.
    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, DecodeError>;
    /// Total duration of the decoded audio if known.
    fn duration(&self) -> Option<Duration> {
        None
    }
    /// Current decoded timestamp position.
    fn elapsed(&self) -> Duration {
        Duration::ZERO
    }
    /// Seeks to `target` and returns the actual timestamp landed on.
    fn seek(&mut self, _target: Duration) -> Result<Duration, DecodeError> {
        Err(DecodeError::SeekFailed("stream is not seekable".into()))
    }
}

/// Driven factory port for opening audio decoders from local files or resolved byte streams.
pub trait DecoderFactoryPort: Send + Sync {
    /// Opens a decoder for a local file path.
    fn open_file(&self, path: &str) -> Result<Box<dyn PcmStreamPort>, DecodeError>;
    /// Opens a decoder for a resolved byte stream.
    fn open_stream(
        &self,
        stream: Box<dyn AudioSourceStream>,
    ) -> Result<Box<dyn PcmStreamPort>, DecodeError>;
}

/// Driven port for cached DJ track analysis, keyed by `amp_domain::analysis_key`.
pub trait AnalysisCachePort: Send + Sync {
    /// The stored analysis for `key`, if any (of any version; callers check it).
    fn load(&self, key: &str) -> Option<amp_domain::TrackAnalysis>;
    /// Stores `analysis` under `key`, replacing an older one.
    fn store(&self, key: &str, analysis: &amp_domain::TrackAnalysis) -> Result<(), StorageError>;
}

/// Driven port for a track's DJ hot cues, keyed by `amp_domain::analysis_key`.
pub trait HotCueStorePort: Send + Sync {
    /// The track's hot cues (all empty when none were saved).
    fn load_hot_cues(&self, key: &str) -> amp_domain::HotCues;
    /// Saves the track's hot cues.
    fn store_hot_cues(&self, key: &str, cues: &amp_domain::HotCues) -> Result<(), StorageError>;
}

/// Driven port for hardware audio output device sinks (`TASK-134`).
///
/// Audio samples flow through the lock-free `rtrb::Producer<f32>` paired with the sink;
/// `AudioSinkPort` governs stream lifecycle, reconfiguration, and latency calculation.
pub trait AudioSinkPort: Send + Sync {
    fn configure(&mut self, sample_rate: u32, channels: u16) -> Result<(), AudioSinkError>;
    fn start(&mut self) -> Result<(), AudioSinkError>;
    fn pause(&mut self) -> Result<(), AudioSinkError>;
    fn stop(&mut self) -> Result<(), AudioSinkError>;
    fn get_buffer_latency(&self) -> Duration;
    fn latency(&self) -> Duration {
        self.get_buffer_latency()
    }
}

/// Driven port for embedded library database and index storage.
#[async_trait]
pub trait LibraryStoragePort: Send + Sync {
    async fn store_track(&self, track: &Track) -> Result<(), StorageError>;
    async fn fetch_all(&self) -> Result<Vec<Track>, StorageError>;
    async fn remove_track(&self, id: &TrackId) -> Result<(), StorageError>;
    async fn search_cached(&self, text_query: &str) -> Result<Vec<Track>, StorageError>;
}

use std::sync::atomic::{compiler_fence, Ordering};

/// Memory-guarded secret string container that redacts `Debug`/`Display` and zeroes its
/// buffer on `Drop` (`TASK-151`).
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString {
    inner: Vec<u8>,
}

impl SecretString {
    /// Wraps a string slice into a protected [`SecretString`] container.
    #[must_use]
    pub fn new(secret: &str) -> Self {
        Self {
            inner: secret.as_bytes().to_vec(),
        }
    }

    /// Exposes secret contents as a UTF-8 string slice.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        std::str::from_utf8(&self.inner).unwrap_or("")
    }

    /// Length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the secret is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Explicitly zeroes the underlying byte buffer.
    pub fn zeroize(&mut self) {
        self.inner.fill(0);
        compiler_fence(Ordering::SeqCst);
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretString(***REDACTED***)")
    }
}

impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***REDACTED***")
    }
}

impl PartialEq<&str> for SecretString {
    fn eq(&self, other: &&str) -> bool {
        self.expose_secret() == *other
    }
}

impl PartialEq<str> for SecretString {
    fn eq(&self, other: &str) -> bool {
        self.expose_secret() == other
    }
}

/// Decrypted secret value retrieved from `CredentialVaultPort` (`TASK-151`).
pub type VaultSecret = SecretString;

/// Driven port for secure credential and secret token storage.
#[async_trait]
pub trait CredentialVaultPort: Send + Sync {
    async fn store_secret(&self, key: &str, secret: &str) -> Result<(), VaultError>;
    async fn get_secret(&self, key: &str) -> Result<Option<SecretString>, VaultError>;
    async fn delete_secret(&self, key: &str) -> Result<(), VaultError>;
}
