//! Native Desktop GUI Adapter for Multiplay Audiophile Player.
//! Provides retro Winamp-styled modular floating windows with magnetic snapping.

#![forbid(unsafe_code)]

pub mod about;
pub mod app;
pub mod app_icon;
pub mod app_paths;
pub mod app_render;
pub mod app_state;
pub mod chrome;
pub mod dialog_jobs;
pub mod dj_console;
pub mod dj_controls;
pub mod dj_mode;
pub mod dj_session;
pub mod dj_strip;
pub mod dj_waveform;
pub mod docking;
pub mod engine_factory;
pub mod equalizer_widget;
pub mod file_dialog;
pub mod layout_manager;
pub mod main_window;
pub mod media_session;
pub mod playback_notice;
pub mod playlist_import;
pub mod playlist_window;
pub mod qr_widget;
pub mod remote_host;
pub mod remote_lookup;
pub mod remote_manager;
pub mod remote_requests;
pub mod remote_window;
pub mod responsive;
pub mod search_actions;
pub mod search_window;
pub mod settings_panel;
pub mod skin_texture;
pub mod transport;
pub mod viewports;
pub mod visualizer_widget;

pub use amp_core_engine::PlayerEngine;
pub use app::MultiplayGuiApp;
pub use docking::{DockManager, DockRect, WindowId};
pub use engine_factory::{create_capture_engine, create_headless_engine, create_player_engine};
pub use visualizer_widget::VisualizerMode;
