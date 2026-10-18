//! Native Desktop GUI Adapter for Multiplay Audiophile Player.
//! Provides retro Winamp-styled modular floating windows with magnetic snapping.

#![forbid(unsafe_code)]

pub mod app;
pub mod app_paths;
pub mod app_render;
pub mod app_state;
pub mod audio_driver;
pub mod audio_pusher;
pub mod dj_audio;
pub mod dj_panel;
pub mod dj_playback;
pub mod docking;
pub mod equalizer_widget;
pub mod file_dialog;
pub mod layout_manager;
pub mod main_window;
pub mod playback_worker;
pub mod playlist_window;
pub mod qr_widget;
pub mod remote_manager;
pub mod settings_panel;
pub mod skin_texture;
pub mod track_loader;
pub mod viewports;
pub mod visualizer_widget;

pub use app::MultiplayGuiApp;
pub use docking::{DockManager, DockRect, WindowId};
pub use visualizer_widget::VisualizerMode;
