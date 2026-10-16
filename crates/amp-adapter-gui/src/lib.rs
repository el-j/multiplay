//! Native Desktop GUI Adapter for Multiplay Audiophile Player.
//! Provides retro Winamp-styled modular floating windows with magnetic snapping.

#![forbid(unsafe_code)]

pub mod app;
pub mod app_render;
pub mod app_state;
pub mod audio_driver;
pub mod dj_panel;
pub mod docking;
pub mod equalizer_widget;
pub mod main_window;
pub mod playlist_window;
pub mod settings_panel;
pub mod visualizer_widget;

pub use app::MultiplayGuiApp;
pub use docking::{DockManager, DockRect, WindowId};
