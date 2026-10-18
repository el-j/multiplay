//! Filesystem locations used by the GUI, injected so tests never touch the user's home.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// Where the GUI reads and writes its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// Session playlist and paired remote devices.
    pub data_dir: PathBuf,
    /// Window layout JSON file.
    pub layout_file: PathBuf,
    /// Folder scanned by "Scan ~/Music" and on first launch; `None` disables scanning.
    pub music_dir: Option<PathBuf>,
}

impl AppPaths {
    /// Locations under the user's home directory (`~/.multiplay`, `~/.config/multiplay`, `~/Music`).
    #[must_use]
    pub fn for_user() -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        Self {
            data_dir: home.join(".multiplay"),
            layout_file: home.join(".config/multiplay/window_layout.json"),
            music_dir: Some(home.join("Music")),
        }
    }

    /// Self-contained locations below `root`, with music scanning disabled. Used by tests.
    #[must_use]
    pub fn in_dir(root: &Path) -> Self {
        Self {
            data_dir: root.join("data"),
            layout_file: root.join("config/window_layout.json"),
            music_dir: None,
        }
    }

    /// Session playlist snapshot file.
    #[must_use]
    pub fn session_file(&self) -> PathBuf {
        self.data_dir.join("session_playlist.json")
    }

    /// Hashed session secrets of paired remote devices.
    #[must_use]
    pub fn remote_sessions_file(&self) -> PathBuf {
        self.data_dir.join("remote_sessions.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_stay_inside_root() {
        let root = Path::new("/tmp/multiplay-test-root");
        let paths = AppPaths::in_dir(root);
        assert!(paths.session_file().starts_with(root));
        assert!(paths.remote_sessions_file().starts_with(root));
        assert!(paths.layout_file.starts_with(root));
        assert!(paths.music_dir.is_none());
    }
}
