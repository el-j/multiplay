//! Audiophile DSP processing chain: Polyphase Sinc resampling, 10-Band EQ, crossfading, and FFT spectrum visualizer.

pub mod biquad;
pub mod crossfade;
pub mod equalizer;
pub mod loudness;
pub mod resampler;
pub mod visualizer;

pub use biquad::BiquadFilter;
pub use crossfade::{CrossfadeCurve, Crossfader};
pub use equalizer::{GraphicEqualizer, EQ_FREQUENCIES};
pub use loudness::{LoudnessConfig, PureVolumeNormalizer};
pub use resampler::PureResampler;
pub use visualizer::SpectrumVisualizer;
