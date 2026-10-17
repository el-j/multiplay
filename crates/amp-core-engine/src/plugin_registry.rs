//! Safe Pure-Rust Plugin Host and Capability Registry.
//! Tracks in-tree and sandboxed pure-Rust plugins without unsafe native C shared-library loading.

#![forbid(unsafe_code)]

use amp_ports::plugin::{AmpAudioPlugin, PluginCapability, PluginManifest};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn AmpAudioPlugin>>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a pure-Rust audio plugin into the host registry.
    pub async fn register_plugin(&self, plugin: Arc<dyn AmpAudioPlugin>) -> Result<(), String> {
        let manifest = plugin.manifest();
        let mut map = self.plugins.write().await;
        if map.contains_key(&manifest.id) {
            return Err(format!("Plugin '{}' is already registered", manifest.id));
        }
        map.insert(manifest.id, plugin);
        Ok(())
    }

    /// Unregisters a plugin by its unique identifier.
    pub async fn unregister_plugin(&self, plugin_id: &str) -> Option<Arc<dyn AmpAudioPlugin>> {
        let mut map = self.plugins.write().await;
        map.remove(plugin_id)
    }

    /// Retrieves all registered plugin manifests.
    pub async fn list_manifests(&self) -> Vec<PluginManifest> {
        let map = self.plugins.read().await;
        map.values().map(|p| p.manifest()).collect()
    }

    /// Queries all plugins providing a specific capability.
    pub async fn list_by_capability(
        &self,
        capability: PluginCapability,
    ) -> Vec<Arc<dyn AmpAudioPlugin>> {
        let map = self.plugins.read().await;
        map.values()
            .filter(|p| p.manifest().capabilities.contains(&capability))
            .cloned()
            .collect()
    }

    pub async fn count(&self) -> usize {
        let map = self.plugins.read().await;
        map.len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockReverbPlugin;

    #[async_trait::async_trait]
    impl AmpAudioPlugin for MockReverbPlugin {
        fn manifest(&self) -> PluginManifest {
            PluginManifest {
                id: "org.multiplay.reverb".into(),
                name: "Studio Reverb".into(),
                version: "1.0.0".into(),
                author: "DSP Team".into(),
                capabilities: vec![PluginCapability::DspProcessor],
            }
        }
    }

    struct MockVisualizerPlugin;

    #[async_trait::async_trait]
    impl AmpAudioPlugin for MockVisualizerPlugin {
        fn manifest(&self) -> PluginManifest {
            PluginManifest {
                id: "org.multiplay.milkdrop_mini".into(),
                name: "MilkDrop Mini".into(),
                version: "0.9.0".into(),
                author: "TUI Team".into(),
                capabilities: vec![PluginCapability::VisualizerEngine],
            }
        }
    }

    #[tokio::test]
    async fn test_plugin_registry_capabilities() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.count().await, 0);

        let reverb = Arc::new(MockReverbPlugin);
        let visualizer = Arc::new(MockVisualizerPlugin);

        registry.register_plugin(reverb).await.unwrap();
        registry.register_plugin(visualizer).await.unwrap();
        assert_eq!(registry.count().await, 2);

        // Duplicate registration fails safely
        let duplicate = Arc::new(MockReverbPlugin);
        assert!(registry.register_plugin(duplicate).await.is_err());

        // Query by capability
        let dsp_plugins = registry
            .list_by_capability(PluginCapability::DspProcessor)
            .await;
        assert_eq!(dsp_plugins.len(), 1);
        assert_eq!(dsp_plugins[0].manifest().name, "Studio Reverb");

        let viz_plugins = registry
            .list_by_capability(PluginCapability::VisualizerEngine)
            .await;
        assert_eq!(viz_plugins.len(), 1);
        assert_eq!(viz_plugins[0].manifest().name, "MilkDrop Mini");

        let source_plugins = registry
            .list_by_capability(PluginCapability::AudioSourceProvider)
            .await;
        assert_eq!(source_plugins.len(), 0);

        // Unregister
        let removed = registry.unregister_plugin("org.multiplay.reverb").await;
        assert!(removed.is_some());
        assert_eq!(registry.count().await, 1);
    }
}
