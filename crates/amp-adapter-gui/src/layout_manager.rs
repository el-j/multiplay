//! Modular window layout serialization and multi-viewport docking synchronization.

#![forbid(unsafe_code)]

use crate::docking::{DockManager, WindowId};
use std::path::Path;

/// Saves modular window layout to `file`.
pub fn save_layout(dock_manager: &DockManager, file: &Path) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(dock_manager) {
        let _ = std::fs::write(file, json);
    }
}

/// Loads modular window layout from `file` if present.
pub fn load_layout(file: &Path) -> Option<DockManager> {
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
