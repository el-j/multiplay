//! Stem separation engine registry and opt-in model download manager.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Supported stem separation engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StemEngineType {
    /// Zero-latency pure-Rust Linkwitz-Riley 4th-order phase-aligned pseudo-stems.
    PhaseAlignedLr4,
    /// Deep learning neural network stem separator (opt-in external/sidecar model).
    DemucsV4,
}

impl StemEngineType {
    /// Human-readable name of the engine.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::PhaseAlignedLr4 => "Phase-Aligned LR4 Filterbank (Built-in)",
            Self::DemucsV4 => "Demucs v4 Neural Separation",
        }
    }

    /// Whether this engine requires an offline download.
    #[must_use]
    pub const fn requires_download(&self) -> bool {
        match self {
            Self::PhaseAlignedLr4 => false,
            Self::DemucsV4 => true,
        }
    }

    /// Approximate download size in bytes.
    #[must_use]
    pub const fn download_size_bytes(&self) -> u64 {
        match self {
            Self::PhaseAlignedLr4 => 0,
            Self::DemucsV4 => 125_829_120, // ~120 MB
        }
    }
}

/// Status of a stem separation model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StemModelStatus {
    /// Ready for instant use.
    Ready,
    /// Requires user opt-in before downloading model weights.
    RequiresOptIn { download_size_bytes: u64 },
    /// Currently downloading model weights.
    Downloading { progress: f32 },
    /// Disabled by user configuration.
    Disabled,
}

/// Manager tracking model statuses, opt-in preferences, and downloads.
#[derive(Debug, Clone, Default)]
pub struct StemModelManager {
    demucs_opted_in: bool,
    demucs_downloaded: bool,
}

impl StemModelManager {
    /// Creates a new model manager with built-in LR4 active and external models gated.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks current status of the specified stem engine.
    #[must_use]
    pub fn status(&self, engine: StemEngineType) -> StemModelStatus {
        match engine {
            StemEngineType::PhaseAlignedLr4 => StemModelStatus::Ready,
            StemEngineType::DemucsV4 => {
                if !self.demucs_opted_in {
                    StemModelStatus::RequiresOptIn {
                        download_size_bytes: engine.download_size_bytes(),
                    }
                } else if self.demucs_downloaded {
                    StemModelStatus::Ready
                } else {
                    StemModelStatus::RequiresOptIn {
                        download_size_bytes: engine.download_size_bytes(),
                    }
                }
            }
        }
    }

    /// Sets opt-in state for an engine.
    pub fn set_opt_in(&mut self, engine: StemEngineType, opt_in: bool) {
        if let StemEngineType::DemucsV4 = engine {
            self.demucs_opted_in = opt_in;
        }
    }

    /// Checks if user has opted in to the specified engine.
    #[must_use]
    pub fn is_opted_in(&self, engine: StemEngineType) -> bool {
        match engine {
            StemEngineType::PhaseAlignedLr4 => true,
            StemEngineType::DemucsV4 => self.demucs_opted_in,
        }
    }

    /// Simulates or records model download completion.
    pub fn set_downloaded(&mut self, engine: StemEngineType, downloaded: bool) {
        if let StemEngineType::DemucsV4 = engine {
            self.demucs_downloaded = downloaded;
        }
    }
}
