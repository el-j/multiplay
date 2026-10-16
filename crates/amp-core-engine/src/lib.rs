//! Core engine orchestrator for multiplay.
//! Provides audio buffer precaching, DSP audio transformations, and event routing.

#![forbid(unsafe_code)]

pub mod config_store;
pub mod dispatcher;
pub mod dj;
pub mod downloader;
pub mod dsp;
pub mod events;
pub mod filtered_search;
pub mod playback;
pub mod plugin_registry;
pub mod precache;
pub mod skin;

pub use config_store::ConfigStore;
pub use dispatcher::EventBus;
pub use dj::{
    CrossfaderCurveProfile, DeckChannelState, DeckId, DjCrossfader, KillIsolatorEq,
    RealtimeBpmDetector, WsolaTimeStretcher,
};
pub use downloader::OfflineTrackDownloader;
pub use dsp::{
    BiquadFilter, CrossfadeCurve, Crossfader, GraphicEqualizer, LoudnessConfig, PureResampler,
    PureVolumeNormalizer, SpectrumVisualizer, EQ_FREQUENCIES,
};
pub use events::{PlayerCommand, PlayerEvent};
pub use filtered_search::FilteredSearchCoordinator;
pub use playback::{RepeatMode, ShuffleMode, SmartQueueManager};
pub use plugin_registry::PluginRegistry;
pub use precache::SharedPrecacheBuffer;
pub use skin::{SkinLoader, WinampTheme};
