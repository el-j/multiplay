//! Core engine orchestrator for multiplay.
//! Provides audio buffer precaching, DSP audio transformations, and event routing.

#![forbid(unsafe_code)]

pub mod config_store;
pub mod diagnostics;
pub mod dispatcher;
pub mod dj;
pub mod downloader;
pub mod dsp;
pub mod events;
pub mod filtered_search;
pub mod library_index;
pub mod playback;
pub mod player;
pub mod plugin_registry;
pub mod precache;
pub mod search_rank;
pub mod skin;

pub use config_store::ConfigStore;
pub use dispatcher::EventBus;
pub use dj::{
    CrossfaderCurveProfile, DeckChannelState, DeckId, DjCrossfader, KillIsolatorEq,
    RealtimeBpmDetector, WsolaTimeStretcher,
};
pub use downloader::{OfflineTrackDownloader, TrackDownloadService};
pub use dsp::{
    BiquadFilter, CrossfadeCurve, Crossfader, GraphicEqualizer, LoudnessConfig, PlaybackDspChain,
    PureResampler, PureVolumeNormalizer, SpectrumVisualizer, EQ_FREQUENCIES,
};
pub use events::{PlayerCommand, PlayerErrorKind, PlayerEvent};
pub use filtered_search::FilteredSearchCoordinator;
pub use playback::{
    ComfortSilenceGenerator, GaplessConfig, GaplessTrackScheduler, RepeatMode, ShuffleMode,
    SmartQueueManager,
};
pub use player::{DjLink, DjRequest, EngineQueue, PlayerEngine, PlayerEngineConfig};
pub use plugin_registry::PluginRegistry;
pub use precache::SharedPrecacheBuffer;
pub use search_rank::{normalize_string, rank_and_deduplicate, score_track};
pub use skin::{SkinLoader, WinampSkin, WinampTheme};
