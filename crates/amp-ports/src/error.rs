#![forbid(unsafe_code)]

use amp_domain::TrackId;
use thiserror::Error;

/// High-level unified error type across all hexagonal port boundaries.
#[derive(Debug, Error)]
pub enum PortError {
    #[error("Audio sink error: {0}")]
    AudioSink(#[from] AudioSinkError),

    #[error("Provider error: {0}")]
    Provider(#[from] crate::driven::ProviderError),

    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Credential vault error: {0}")]
    Vault(#[from] VaultError),

    #[error("Playlist error: {0}")]
    Playlist(#[from] PlaylistError),

    #[error("Download error: {0}")]
    Download(#[from] DownloadError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal engine error: {0}")]
    Internal(String),
}

/// Errors occurring while opening, decoding, or seeking an audio stream.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// Stream buffer underflow while waiting for progressive network chunks (non-fatal).
    #[error("Stream buffer underflow: waiting for data")]
    WouldBlock,

    /// Audio file or stream could not be opened.
    #[error("Failed to open audio source: {0}")]
    OpenFailed(String),

    /// Container format or audio codec is unrecognized or unsupported.
    #[error("Unsupported audio format or codec: {0}")]
    UnsupportedFormat(String),

    /// Packet read or frame decode failure.
    #[error("Decode error: {0}")]
    CorruptStream(String),

    /// Stream seek failed or is unsupported.
    #[error("Seek failed: {0}")]
    SeekFailed(String),
}

/// Errors occurring at the audio output device sink boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AudioSinkError {
    #[error("Audio hardware device not found: {0}")]
    DeviceNotFound(String),

    #[error("Unsupported audio channel or sample rate format: {0}")]
    UnsupportedFormat(String),

    #[error("Audio output stream failure: {0}")]
    StreamError(String),

    #[error("Buffer underflow or starvation detected")]
    Underflow,

    #[error("Buffer overrun: hardware consumed slower than producer")]
    Overrun,
}

/// Errors occurring during storage transactions and entity persistence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StorageError {
    #[error("Storage database transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Serialization / deserialization failure: {0}")]
    Serialization(String),

    #[error("Requested record not found in storage: {0}")]
    NotFound(String),

    #[error("Storage backend I/O failure: {0}")]
    Io(String),
}

/// Errors occurring within the secure credential vault.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VaultError {
    #[error("Key not found in vault: {0}")]
    NotFound(String),

    #[error("Encryption or decryption failed: {0}")]
    CryptoError(String),

    #[error("Storage access error in vault: {0}")]
    Storage(String),
}

/// Errors occurring during playlist or queue manipulation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlaylistError {
    #[error("Track not found in playlist: {0:?}")]
    TrackNotFound(TrackId),

    #[error("Playlist index {index} out of bounds (len {len})")]
    IndexOutOfBounds { index: usize, len: usize },

    #[error("Engine unavailable or shut down")]
    EngineTerminated,
}

/// Errors occurring during offline track download and tagging.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadError {
    #[error("No stream resolver handles source")]
    UnsupportedSource,

    #[error("Provider stream error: {0}")]
    Provider(String),

    #[error("Filesystem I/O error: {0}")]
    Io(String),

    #[error("Metadata tagging error: {0}")]
    Tagging(String),

    #[error("Download job not found: {0:?}")]
    NotFound(TrackId),
}
