# Pure-Rust Plugin System & Skinning Engine

This document defines:
1. **Classic Skin Engine**: Native decompression of classic `.wsz` archives (ZIP format) and parsing of Winamp 1.x / 2.x bitmap sprite sheets into custom terminal palettes and GUI textures.
2. **Safe Pure-Rust Plugin Architecture**: Isolation boundary allowing third-party audio decoders, streaming sources, DSP modules, and visualizers without dynamic C linkage or unsafe raw shared library loading.

---

## 1. Winamp Classic Skin Archive Decompressor & Palette Extractor

Winamp `.wsz` skins are standard `.zip` files containing `.bmp` sprite sheets (`MAIN.BMP`, `CETHUMB.BMP`, `PLEDIT.BMP`) and text configuration files (`PLEDIT.TXT`, `VISCOLOR.TXT`).

```
       ┌────────────────────────────────────────────────────────┐
       │               Winamp Skin Archive (.wsz)               │
       │       (Read via pure-Rust `zip` crate in memory)       │
       └──────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
       ┌────────────────────────────────────────────────────────┐
       │             Pure-Rust BMP & Palette Parser             │
       │  • VISCOLOR.TXT: RGB colors for 16-bar spectrum meters │
       │  • PLEDIT.TXT: Custom fonts and playlist colors        │
       │  • MAIN.BMP: Transport buttons, numbers, LED digits    │
       └──────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
       ┌────────────────────────────────────────────────────────┐
       │               Runtime Theme State Object               │
       │    (Used across Ratatui TUI & egui GUI renderers)      │
       └────────────────────────────────────────────────────────┘
```

### 1.1. Pure-Rust Skin Asset Loader (`amp-core-engine/src/skin.rs`)

```rust
// crates/amp-core-engine/src/skin.rs
use ratatui::style::Color;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct WinampTheme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub text_accent: Color,
    pub vfd_time_color: Color,
    pub spectrum_colors: Vec<Color>, // 16 discrete colors (green -> yellow -> red)
    pub transport_active: Color,
}

impl Default for WinampTheme {
    fn default() -> Self {
        Self {
            name: "Classic Winamp 1.x Default".into(),
            background: Color::Rgb(24, 24, 24),
            foreground: Color::Rgb(180, 180, 180),
            text_accent: Color::Rgb(0, 255, 0),
            vfd_time_color: Color::Rgb(0, 235, 0),
            spectrum_colors: vec![
                Color::Rgb(0, 255, 0),
                Color::Rgb(50, 255, 0),
                Color::Rgb(150, 255, 0),
                Color::Rgb(255, 255, 0),
                Color::Rgb(255, 150, 0),
                Color::Rgb(255, 0, 0),
            ],
            transport_active: Color::Rgb(0, 255, 0),
        }
    }
}

pub struct SkinLoader;

impl SkinLoader {
    /// Loads and parses skin metadata from a raw `.wsz` byte slice
    pub fn load_from_wsz_bytes(bytes: &[u8]) -> Result<WinampTheme, String> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| format!("Invalid ZIP/WSZ: {e}"))?;

        let mut theme = WinampTheme::default();

        // 1. Read viscolor.txt if present (Defines spectrum analyzer gradient)
        if let Ok(mut file) = archive.by_name("viscolor.txt") {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                theme.spectrum_colors = parse_viscolor(&content);
            }
        }

        // 2. Read pledit.txt for playlist font/color definitions
        if let Ok(mut file) = archive.by_name("pledit.txt") {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Some(bg) = parse_ini_color(&content, "NormalBG") {
                    theme.background = bg;
                }
                if let Some(fg) = parse_ini_color(&content, "Normal") {
                    theme.foreground = fg;
                }
            }
        }

        Ok(theme)
    }
}

fn parse_viscolor(content: &str) -> Vec<Color> {
    let mut colors = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse(), parts[1].parse(), parts[2].parse()) {
                colors.push(Color::Rgb(r, g, b));
            }
        }
    }
    if colors.is_empty() {
        WinampTheme::default().spectrum_colors
    } else {
        colors
    }
}

fn parse_ini_color(content: &str, key: &str) -> Option<Color> {
    for line in content.lines() {
        if line.to_lowercase().starts_with(&key.to_lowercase()) {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                let hex = parts[1].trim().trim_start_matches('#');
                if hex.len() == 6 {
                    if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                        return Some(Color::Rgb(
                            ((rgb >> 16) & 0xFF) as u8,
                            ((rgb >> 8) & 0xFF) as u8,
                            (rgb & 0xFF) as u8,
                        ));
                    }
                }
            }
        }
    }
    None
}
```

---

## 2. Pure-Rust Plugin Host Architecture

Loading untrusted native C dynamic libraries (`.dll`, `.so`, `.dylib`) via `libloading` breaks the pure-Rust zero-crash and zero-unsafe guarantee. 

To maintain isolation, safety, and cross-platform binary compatibility, the plugin system offers two paths:
1. **Static In-Tree Plugin Registration (Compile-Time)**: Ideal for official first-party adapters.
2. **WebAssembly Component Model (`wasmtime` sandbox)**: For third-party audio filters, metadata providers, and custom DSP equalizers.

```
                  ┌──────────────────────────────────────────────┐
                  │           Core Hexagonal Engine              │
                  │         (Shared State & CQRS Bus)            │
                  └──────────────────────┬───────────────────────┘
                                         │
                   ┌─────────────────────┴─────────────────────┐
                   │                                           │
                   ▼                                           ▼
   ┌───────────────────────────────┐           ┌───────────────────────────────┐
   │  Static Hexagonal Extensions  │           │   Pure-Rust WASM Sandbox      │
   │  (Compile-time Pure-Rust)     │           │   (wasmtime Component Engine) │
   │  • Bandcamp Resolver          │           │   • Custom Biquad DSP Filters │
   │  • Direct Mesh Peer Protocol  │           │   • Web Scrapers & Metadata   │
   │  • Lofty Metadata Tagger      │           │   • Visualizer Color Shaders  │
   └───────────────────────────────┘           └───────────────────────────────┘
```

### 2.1. Plugin Trait Definitions (`amp-ports/src/plugin.rs`)

```rust
// crates/amp-ports/src/plugin.rs
use crate::driven::{ProviderSearchPort, StreamResolverPort};
use amp_domain::Track;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCapability {
    AudioSourceProvider,
    DspProcessor,
    VisualizerEngine,
    MetadataEnricher,
}

pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub capabilities: Vec<PluginCapability>,
}

#[async_trait]
pub trait AmpAudioPlugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    
    /// Optional hook to register search & streaming capabilities
    fn provider_components(&self) -> Option<(Arc<dyn ProviderSearchPort>, Arc<dyn StreamResolverPort>)> {
        None
    }

    /// Optional hook for in-line real-time sample processing (Equalizers, Reverbs)
    fn dsp_processor(&mut self) -> Option<&mut dyn DspFilterHook> {
        None
    }
}

pub trait DspFilterHook: Send {
    /// In-place real-time sample transformation on interleaved PCM
    fn process_frame(&mut self, samples: &mut [f32], channels: u16, sample_rate: u32);
}
```

### 2.2. Plugin Registry & Sandbox Coordinator

```rust
// crates/amp-core-engine/src/plugin_registry.rs
use amp_ports::plugin::{AmpAudioPlugin, PluginCapability};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PluginRegistry {
    loaded_plugins: Arc<RwLock<HashMap<String, Arc<dyn AmpAudioPlugin>>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            loaded_plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_plugin(&self, plugin: Arc<dyn AmpAudioPlugin>) -> Result<(), String> {
        let manifest = plugin.manifest();
        let mut registry = self.loaded_plugins.write().await;
        
        if registry.contains_key(&manifest.id) {
            return Err(format!("Plugin with ID '{}' already loaded", manifest.id));
        }

        println!("󰐱 Loaded Pure-Rust Plugin: {} v{}", manifest.name, manifest.version);
        registry.insert(manifest.id, plugin);
        Ok(())
    }

    pub async fn list_by_capability(&self, capability: PluginCapability) -> Vec<Arc<dyn AmpAudioPlugin>> {
        let registry = self.loaded_plugins.read().await;
        registry
            .values()
            .filter(|p| p.manifest().capabilities.contains(&capability))
            .cloned()
            .collect()
    }
}
```

---

## 3. Keyboard & Mouse Shortcuts for Dense Winamp Workflow

To match the fast tactile controls of classic hardware players and Winamp:

| Key Binding | Function | Target Component |
| :--- | :--- | :--- |
| `z` | Previous Track | Main Console |
| `x` | Play / Restart | Main Console |
| `c` | Pause / Unpause | Main Console |
| `v` | Stop Playback | Main Console |
| `b` | Next Track | Main Console |
| `j` or `Ctrl + F` | Jump to File (Omnibar Search) | Library / Search Modal |
| `Alt + W` | Toggle Video Window Tear-Off | Video Stream Canvas |
| `Alt + E` | Toggle 10-Band EQ Window | Equalizer Dock |
| `Alt + P` | Toggle Playlist Editor Window | Playlist Dock |
| `Tab` | Cycle Magnetic Dock Focus | Docking Window Manager |
| `+` / `-` | Hardware Volume Boost / Cut | Hardware Sink |
| `[` / `]` | 10-Second Fast Seek Rewind / Forward | Audio Precaching Stream |