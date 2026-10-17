//! Modular window layout serialization and multi-viewport docking synchronization.

#![forbid(unsafe_code)]

use crate::docking::{DockManager, WindowId};
use std::path::PathBuf;

/// Saves modular window layout to configuration directory.
pub fn save_layout(dock_manager: &DockManager) {
    if let Some(home) = std::env::var_os("HOME") {
        let config_dir = PathBuf::from(home).join(".config/multiplay");
        let _ = std::fs::create_dir_all(&config_dir);
        if let Ok(json) = serde_json::to_string_pretty(dock_manager) {
            let _ = std::fs::write(config_dir.join("window_layout.json"), json);
        }
    }
}

/// Loads modular window layout from configuration directory if present.
pub fn load_layout() -> Option<DockManager> {
    let home = std::env::var_os("HOME")?;
    let file = PathBuf::from(home).join(".config/multiplay/window_layout.json");
    let json = std::fs::read_to_string(file).ok()?;
    serde_json::from_str::<DockManager>(&json).ok()
}

/// Synchronizes positions of all transitively docked satellite windows when parent window moves.
pub fn sync_child_viewport_positions(dock_manager: &DockManager, ctx: &eframe::egui::Context) {
    for (&child, &(parent, _)) in &dock_manager.dock_parent {
        if parent == WindowId::Main && dock_manager.is_visible(child) {
            let rect = dock_manager.get_rect(child);
            let tag = match child {
                WindowId::Equalizer => "win_eq",
                WindowId::Playlist => "win_playlist",
                WindowId::Settings => "win_settings",
                WindowId::DjConsole => "win_dj",
                WindowId::Main => continue,
            };
            let id = eframe::egui::ViewportId::from_hash_of(tag);
            let pos = eframe::egui::Pos2::new(rect.x, rect.y);
            ctx.send_viewport_cmd_to(id, eframe::egui::ViewportCommand::OuterPosition(pos));
        }
    }
}
