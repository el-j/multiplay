//! Core engine orchestrator for multiplay.
//! Provides audio buffer precaching, DSP audio transformations, and event routing.

#![forbid(unsafe_code)]

pub mod dispatcher;
pub mod downloader;
pub mod dsp;
pub mod events;
pub mod filtered_search;
pub mod plugin_registry;
pub mod precache;
pub mod skin;

pub use dispatcher::EventBus;
pub use downloader::OfflineTrackDownloader;
pub use dsp::{BiquadFilter, GraphicEqualizer, PureResampler, SpectrumVisualizer, EQ_FREQUENCIES};
pub use events::{PlayerCommand, PlayerEvent};
pub use filtered_search::FilteredSearchCoordinator;
pub use plugin_registry::PluginRegistry;
pub use precache::SharedPrecacheBuffer;
pub use skin::{SkinLoader, WinampTheme};
