//! Playlists and CUE sheets added by drop, dialog or folder scan (TASK-232): M3U/M3U8, PLS and
//! XSPF with paths relative to the playlist's folder, and CUE sheets as one row per song.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use amp_adapter_storage::{import_cue, import_m3u8_with_base, import_pls, import_xspf};
use std::path::Path;

/// Extensions of files that list tracks rather than hold audio.
pub const LIST_EXTENSIONS: [&str; 5] = ["m3u", "m3u8", "pls", "xspf", "cue"];

impl MultiplayGuiApp {
    /// Adds the tracks listed in `path`; `false` when it is not a playlist or CUE sheet.
    pub(crate) fn import_list_file(&mut self, path: &Path) -> bool {
        let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .filter(|e| LIST_EXTENSIONS.contains(&e.as_str()))
        else {
            return false;
        };
        let base = path.parent();
        let text = || {
            std::fs::read(path)
                .ok()
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        };
        let tracks = match ext.as_str() {
            "pls" => text().map(|t| import_pls(&t, base)),
            "xspf" => text().map(|t| import_xspf(&t, base)),
            "cue" => import_cue(path).ok(),
            _ => text().map(|t| import_m3u8_with_base(&t, base)),
        };
        self.playlist.extend(tracks.unwrap_or_default());
        true
    }
}
