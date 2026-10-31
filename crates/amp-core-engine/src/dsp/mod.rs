//! Audiophile DSP processing chain: Polyphase Sinc resampling, 10-Band EQ, crossfading, and FFT spectrum visualizer.

pub mod biquad;
pub mod chain;
pub mod crossfade;
pub mod equalizer;
pub mod headphone_crossfeed;
pub mod limiter;
pub mod loudness;
pub mod resampler;
pub mod visualizer;

pub use biquad::BiquadFilter;
pub use chain::PlaybackDspChain;
pub use crossfade::{CrossfadeCurve, Crossfader};
pub use equalizer::{GraphicEqualizer, EQ_FREQUENCIES};
pub use headphone_crossfeed::{CrossfeedProfile, HeadphoneCrossfeed};
pub use limiter::{LimiterConfig, LookaheadTruePeakLimiter};
pub use loudness::{Bs1770Analyzer, LoudnessConfig, LoudnessMode, PureVolumeNormalizer};
pub use resampler::PureResampler;
pub use visualizer::SpectrumVisualizer;
