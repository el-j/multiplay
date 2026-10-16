//! Playback queue, shuffle, and transition modules.

#![forbid(unsafe_code)]

pub mod shuffle;

pub use shuffle::{RepeatMode, ShuffleMode, SmartQueueManager};
