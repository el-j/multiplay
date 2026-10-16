#![forbid(unsafe_code)]

use thiserror::Error;

/// High-level unified error type across all hexagonal port boundaries.
#[derive(Debug, Error)]
pub enum PortError {
    #[error("Audio sink error: {0}")]
    AudioSink(#[from] AudioSinkError),

    #[error("Provider error: {0}")]
    Provider(#[from] crate::driven::ProviderError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Credential vault error: {0}")]
    Vault(#[from] VaultError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal engine error: {0}")]
    Internal(String),
}

/// Errors occurring at the audio output device sink boundary.
#[derive(Debug, Error)]
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
#[derive(Debug, Error)]
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
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Key not found in vault: {0}")]
    NotFound(String),

    #[error("Encryption or decryption failed: {0}")]
    CryptoError(String),

    #[error("Storage access error in vault: {0}")]
    Storage(String),
}
