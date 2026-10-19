//! Multi-Deck professional DJ mixing engine, BPM detection, and WSOLA time-stretching.

#![forbid(unsafe_code)]

pub mod beat_detector;
pub mod deck;
pub mod engine;
pub mod mixer;
pub mod wsola;

pub use beat_detector::RealtimeBpmDetector;
pub use deck::{DeckChannelState, DeckId, KillIsolatorEq};
pub use mixer::DualDeckMixer;
pub use wsola::{CrossfaderCurveProfile, DjCrossfader, WsolaTimeStretcher};
