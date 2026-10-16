//! Atomic configuration storage and hot-reload synchronization engine.
//! Manages disk persistence via atomic swap and provides live reader handles for the audio thread.

#![forbid(unsafe_code)]

use crate::events::PlayerEvent;
use crate::EventBus;
use amp_domain::AppConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Atomic configuration store engine with live in-memory synchronization.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
    config: Arc<RwLock<AppConfig>>,
    event_bus: Option<Arc<EventBus>>,
}

impl ConfigStore {
    /// Loads configuration from the specified path, initializing default if absent or corrupted.
    pub fn load_or_default(path: impl Into<PathBuf>, event_bus: Option<Arc<EventBus>>) -> Self {
        let path = path.into();
        let loaded = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => basic_toml::from_str::<AppConfig>(&content).unwrap_or_default(),
                Err(_) => AppConfig::default(),
            }
        } else {
            AppConfig::default()
        };

        Self {
            path,
            config: Arc::new(RwLock::new(loaded)),
            event_bus,
        }
    }

    /// Exposes the live shared configuration handle for concurrent audio threads.
    #[must_use]
    pub fn shared_config(&self) -> Arc<RwLock<AppConfig>> {
        Arc::clone(&self.config)
    }

    /// Returns a cloned snapshot of the active configuration.
    #[must_use]
    pub fn current(&self) -> AppConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// Mutates the active configuration in-memory and atomically persists to disk.
    pub fn update<F>(&self, mutate: F) -> Result<(), String>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut new_config = {
            let guard = self.config.read().map_err(|e| e.to_string())?;
            guard.clone()
        };

        mutate(&mut new_config);

        // 1. Serialize to TOML
        let toml_str = basic_toml::to_string(&new_config).map_err(|e| e.to_string())?;

        // 2. Ensure parent directories exist
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // 3. Write to temporary file in same directory
        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, toml_str).map_err(|e| e.to_string())?;

        // 4. Atomic rename replacement
        fs::rename(&tmp_path, &self.path).map_err(|e| e.to_string())?;

        // 5. Update live memory state
        {
            let mut guard = self.config.write().map_err(|e| e.to_string())?;
            *guard = new_config.clone();
        }

        // 6. Notify event bus
        if let Some(bus) = &self.event_bus {
            bus.emit_event(PlayerEvent::ConfigChanged(Box::new(new_config)));
        }

        Ok(())
    }

    /// Default canonical configuration directory for current platform.
    #[must_use]
    pub fn default_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Path::new(&home).join(".config").join("multiplay").join("multiplay.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_store_atomic_update() {
        let tmp_dir = std::env::temp_dir().join(format!("multiplay_test_{}", std::process::id()));
        let config_file = tmp_dir.join("test_config.toml");

        let store = ConfigStore::load_or_default(&config_file, None);
        assert_eq!(store.current().audio.sample_rate, 48000);

        // Update
        store
            .update(|cfg| {
                cfg.audio.sample_rate = 96000;
                cfg.ui.compact_mode = true;
            })
            .expect("Update failed");

        assert_eq!(store.current().audio.sample_rate, 96000);
        assert!(store.current().ui.compact_mode);

        // Reload from disk to verify atomic file persistence
        let reloaded = ConfigStore::load_or_default(&config_file, None);
        assert_eq!(reloaded.current().audio.sample_rate, 96000);
        assert!(reloaded.current().ui.compact_mode);

        // Clean up
        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
