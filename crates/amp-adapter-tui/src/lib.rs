//! Winamp 1.x Dense Console TUI, Multi-Window Docking, and Omnibar Search Adapters.

#![forbid(unsafe_code)]

pub mod dense_console;
pub mod docking;
pub mod equalizer_view;
pub mod playlist_view;
pub mod search_view;
pub mod video_view;

pub use dense_console::{DenseWinampConsole, FACEPLATE_HEIGHT, FACEPLATE_WIDTH};
pub use docking::{DockingWindowManager, WindowId, WindowSlot, SNAP_THRESHOLD_CELLS};
pub use equalizer_view::EqualizerWidget;
pub use playlist_view::PlaylistWidget;
pub use search_view::{SearchViewModal, SearchViewState};
pub use video_view::{VideoFrameBuffer, VideoWindowView};
