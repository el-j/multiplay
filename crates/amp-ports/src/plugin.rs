#![forbid(unsafe_code)]

use crate::driven::{ProviderSearchPort, StreamResolverPort};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginCapability {
    AudioSourceProvider,
    DspProcessor,
    VisualizerEngine,
    MetadataEnricher,
}

#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub capabilities: Vec<PluginCapability>,
}

/// Interface for safe pure-Rust plugins (in-tree or WASM sandbox).
#[async_trait]
pub trait AmpAudioPlugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;

    /// Optional hook to register search & streaming capabilities
    fn provider_components(
        &self,
    ) -> Option<(Arc<dyn ProviderSearchPort>, Arc<dyn StreamResolverPort>)> {
        None
    }

    /// Optional hook for in-line real-time sample processing (Equalizers, Reverbs)
    fn dsp_processor(&mut self) -> Option<&mut dyn DspFilterHook> {
        None
    }
}

/// Hook for real-time DSP sample transformation on interleaved PCM audio.
pub trait DspFilterHook: Send {
    fn process_frame(&mut self, samples: &mut [f32], channels: u16, sample_rate: u32);
}
