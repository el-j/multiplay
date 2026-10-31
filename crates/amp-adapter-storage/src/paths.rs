//! Cross-platform application directory resolver and legacy `~/.multiplay` migration (`TASK-147`).
//!
//! Resolves `config_dir`, `data_dir`, `cache_dir`, and `log_dir` across macOS, Linux, and Windows,
//! honors `MULTIPLAY_HOME` for portable / hermetic test mode, works on Windows even when `HOME` is
//! unset, expands `~` in user-configured paths, and performs a one-time idempotent migration from
//! `~/.multiplay/*` and `~/.config/multiplay/*`.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolved platform directories for `multiplay`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Resolves [`AppPaths`] from the current process environment (checking `MULTIPLAY_HOME` first).
    pub fn from_env() -> Self {
        let env_map: HashMap<String, PathBuf> = [
            "MULTIPLAY_HOME",
            "HOME",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
        ]
        .into_iter()
        .filter_map(|k| std::env::var_os(k).map(|v| (k.to_string(), PathBuf::from(v))))
        .collect();
        Self::from_env_map(&env_map)
    }

    /// Creates portable [`AppPaths`] rooted under a single `portable_root` directory.
    pub fn from_portable_root(portable_root: impl AsRef<Path>) -> Self {
        let root = portable_root.as_ref();
        Self {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("logs"),
        }
    }

    /// Resolves [`AppPaths`] deterministically from an environment variable map (used for cross-OS tests).
    pub fn from_env_map(env: &HashMap<String, PathBuf>) -> Self {
        if let Some(portable) = env
            .get("MULTIPLAY_HOME")
            .filter(|p| !p.as_os_str().is_empty())
        {
            return Self::from_portable_root(portable);
        }

        // If Windows APPDATA / LOCALAPPDATA are provided (or on Windows without HOME), use Windows layout.
        if cfg!(target_os = "windows")
            || (env.get("HOME").is_none()
                && (env.contains_key("APPDATA")
                    || env.contains_key("LOCALAPPDATA")
                    || env.contains_key("USERPROFILE")))
        {
            let user_profile = env
                .get("USERPROFILE")
                .cloned()
                .or_else(|| env.get("HOME").cloned())
                .unwrap_or_else(|| std::env::temp_dir().join("multiplay-user"));
            let roaming = env
                .get("APPDATA")
                .cloned()
                .unwrap_or_else(|| user_profile.join("AppData").join("Roaming"));
            let local = env
                .get("LOCALAPPDATA")
                .cloned()
                .unwrap_or_else(|| user_profile.join("AppData").join("Local"));
            return Self {
                config_dir: roaming.join("multiplay").join("config"),
                data_dir: local.join("multiplay").join("data"),
                cache_dir: local.join("multiplay").join("cache"),
                log_dir: local.join("multiplay").join("logs"),
            };
        }

        let home = env
            .get("HOME")
            .cloned()
            .or_else(|| env.get("USERPROFILE").cloned())
            .unwrap_or_else(|| std::env::temp_dir().join("multiplay-user"));

        if cfg!(target_os = "macos")
            && !env.contains_key("XDG_CONFIG_HOME")
            && !env.contains_key("XDG_DATA_HOME")
        {
            let app_support = home
                .join("Library")
                .join("Application Support")
                .join("multiplay");
            return Self {
                config_dir: app_support.clone(),
                data_dir: app_support,
                cache_dir: home.join("Library").join("Caches").join("multiplay"),
                log_dir: home.join("Library").join("Logs").join("multiplay"),
            };
        }

        let config_dir = env
            .get("XDG_CONFIG_HOME")
            .cloned()
            .unwrap_or_else(|| home.join(".config"))
            .join("multiplay");
        let data_dir = env
            .get("XDG_DATA_HOME")
            .cloned()
            .unwrap_or_else(|| home.join(".local").join("share"))
            .join("multiplay");
        let cache_dir = env
            .get("XDG_CACHE_HOME")
            .cloned()
            .unwrap_or_else(|| home.join(".cache"))
            .join("multiplay");
        let log_dir = env
            .get("XDG_STATE_HOME")
            .cloned()
            .unwrap_or_else(|| home.join(".local").join("state"))
            .join("multiplay")
            .join("logs");

        Self {
            config_dir,
            data_dir,
            cache_dir,
            log_dir,
        }
    }

    /// Creates all four application directories if they do not yet exist.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.config_dir)?;
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.cache_dir)?;
        fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }

    /// Canonical path to `config.toml`.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Canonical path to the persistent `library.redb` database (`TASK-148`).
    pub fn library_db_file(&self) -> PathBuf {
        self.data_dir.join("library.redb")
    }

    /// Canonical path to `session_playlist.json`.
    pub fn session_playlist_file(&self) -> PathBuf {
        self.data_dir.join("session_playlist.json")
    }

    /// Canonical path to `window_layout.json`.
    pub fn window_layout_file(&self) -> PathBuf {
        self.config_dir.join("window_layout.json")
    }

    /// Canonical path to the fallback AEAD-encrypted credential vault (`TASK-151`).
    pub fn vault_file(&self) -> PathBuf {
        self.data_dir.join("credentials.vault")
    }

    /// Performs a one-time idempotent migration from `<legacy_home>/.multiplay/*` and
    /// `<legacy_home>/.config/multiplay/*` into `self.config_dir` and `self.data_dir`.
    /// Returns the number of files migrated.
    pub fn migrate_from_legacy_home(&self, legacy_home: &Path) -> std::io::Result<usize> {
        self.ensure_dirs()?;
        let marker = self.data_dir.join(".migrated_v1");
        if marker.exists() {
            return Ok(0);
        }

        let mut migrated = 0usize;
        let dot_multiplay = legacy_home.join(".multiplay");
        let dot_config_multiplay = legacy_home.join(".config").join("multiplay");

        for legacy_dir in [&dot_multiplay, &dot_config_multiplay] {
            if !legacy_dir.is_dir()
                || legacy_dir == &self.config_dir
                || legacy_dir == &self.data_dir
            {
                continue;
            }
            if let Ok(entries) = fs::read_dir(legacy_dir) {
                for entry in entries.flatten() {
                    let src = entry.path();
                    if !src.is_file() {
                        continue;
                    }
                    let Some(name) = src.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    let dest_dir = if name.ends_with(".toml") || name == "window_layout.json" {
                        &self.config_dir
                    } else {
                        &self.data_dir
                    };
                    let dst = dest_dir.join(name);
                    if !dst.exists() && fs::copy(&src, &dst).is_ok() {
                        migrated += 1;
                    }
                }
            }
        }

        let _ = fs::write(&marker, b"migrated=1\n");
        Ok(migrated)
    }
}

/// Expands a leading `~` or `~/` in `raw_path` using `home_override` or the user's home directory.
pub fn expand_tilde(raw_path: &str, home_override: Option<&Path>) -> PathBuf {
    let trimmed = raw_path.trim();
    if trimmed == "~" || trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        let home = home_override
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        if trimmed == "~" {
            home
        } else {
            home.join(&trimmed[2..])
        }
    } else {
        PathBuf::from(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn paths_work_on_windows_without_home_and_respect_multiplay_home() {
        let mut win_env = HashMap::new();
        win_env.insert("USERPROFILE".to_string(), PathBuf::from("C:\\Users\\Alice"));
        win_env.insert(
            "APPDATA".to_string(),
            PathBuf::from("C:\\Users\\Alice\\AppData\\Roaming"),
        );
        win_env.insert(
            "LOCALAPPDATA".to_string(),
            PathBuf::from("C:\\Users\\Alice\\AppData\\Local"),
        );

        let win_paths = AppPaths::from_env_map(&win_env);
        assert!(win_paths.config_dir.to_string_lossy().contains("multiplay"));
        assert!(win_paths.data_dir.to_string_lossy().contains("multiplay"));

        let mut portable_env = win_env.clone();
        portable_env.insert(
            "MULTIPLAY_HOME".to_string(),
            PathBuf::from("/tmp/portable-multiplay"),
        );
        let portable = AppPaths::from_env_map(&portable_env);
        assert_eq!(
            portable.config_dir,
            PathBuf::from("/tmp/portable-multiplay/config")
        );
        assert_eq!(
            portable.library_db_file(),
            PathBuf::from("/tmp/portable-multiplay/data/library.redb")
        );
    }

    #[test]
    fn paths_migrate_legacy_dot_multiplay_and_expand_tilde() {
        let tmp = tempdir().unwrap();
        let legacy_home = tmp.path().join("legacy_home");
        let dot_multiplay = legacy_home.join(".multiplay");
        let dot_config = legacy_home.join(".config").join("multiplay");
        fs::create_dir_all(&dot_multiplay).unwrap();
        fs::create_dir_all(&dot_config).unwrap();

        fs::write(
            dot_multiplay.join("session_playlist.json"),
            b"{\"tracks\":[]}",
        )
        .unwrap();
        fs::write(
            dot_multiplay.join("window_layout.json"),
            b"{\"windows\":[]}",
        )
        .unwrap();
        fs::write(dot_config.join("config.toml"), b"[audio]\n").unwrap();

        let target_root = tmp.path().join("portable_target");
        let paths = AppPaths::from_portable_root(&target_root);
        let migrated_count = paths.migrate_from_legacy_home(&legacy_home).unwrap();
        assert_eq!(migrated_count, 3);
        assert!(paths.session_playlist_file().exists());
        assert!(paths.window_layout_file().exists());
        assert!(paths.config_file().exists());

        // Second run is idempotent and migrates 0 files
        assert_eq!(paths.migrate_from_legacy_home(&legacy_home).unwrap(), 0);

        // Tilde expansion
        let expanded = expand_tilde("~/Music/Library", Some(&legacy_home));
        assert_eq!(expanded, legacy_home.join("Music/Library"));
    }
}
