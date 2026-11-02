//! Stem isolation, separation, and caching for DJ mixing and remix loops (TASK-223).
//!
//! Provides real-time 4-stem separation (Drums, Bass, Melody, Vocals) using
//! Linkwitz-Riley 4th-order (LR4) phase-aligned crossovers with flat magnitude
//! reconstruction, offline batch separation, stem caching, and opt-in model management.

#![forbid(unsafe_code)]

pub mod cache;
pub mod isolator;
pub mod model;
pub mod params;
pub mod separator;

pub use cache::StemCache;
pub use isolator::{
    stem_filter_cutoff, StemIsolator, BASS_DRUMS_HZ, DRUMS_VOCALS_HZ, VOCALS_MELODY_HZ,
};
pub use model::{StemEngineType, StemModelManager, StemModelStatus};
pub use params::{StemDeckParams, StemType, STEM_COUNT};
pub use separator::SeparatedStems;
