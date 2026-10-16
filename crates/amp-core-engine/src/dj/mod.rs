//! Multi-Deck professional DJ mixing engine, BPM detection, and WSOLA time-stretching.

#![forbid(unsafe_code)]

pub mod beat_detector;
pub mod deck;
pub mod wsola;

pub use beat_detector::RealtimeBpmDetector;
pub use deck::{DeckChannelState, DeckId, KillIsolatorEq};
pub use wsola::{CrossfaderCurveProfile, DjCrossfader, WsolaTimeStretcher};
