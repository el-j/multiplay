//! Playback queue, shuffle, and transition modules.

#![forbid(unsafe_code)]

pub mod gapless;
pub mod shuffle;

pub use gapless::{ComfortSilenceGenerator, GaplessConfig, GaplessTrackScheduler};
pub use shuffle::{RepeatMode, ShuffleMode, SmartQueueManager};
