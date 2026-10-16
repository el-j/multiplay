---
id: TASK-043
title: Hierarchical TOML Configuration Domain Schema
planId: PLAN-014
status: completed
role: architecture
verificationCommand: cargo test -p amp-domain test_config_schema
---

# TASK-043: Hierarchical TOML Configuration Domain Schema

## Description
Implement the unified configuration data structures in `crates/amp-domain/src/config.rs`:
1. `AppConfig`: Top-level hierarchical schema with `Default` and `Serialize`/`Deserialize` implementations.
2. Section Schemas:
   - `LibraryConfig`: Indexed directories, file watching toggles, rescan intervals, and supported extensions list.
   - `AudioHardwareConfig`: Output device name, buffer size frames, RAM precache size (MB), target sample rate, and crossfade duration.
   - `StreamingProvidersConfig`: YouTube, Spotify, SoundCloud, Bandcamp toggles, and download cache directory.
   - `P2PNetworkConfig`: Mesh and Soulseek listener ports, upload slots, UPnP/NAT toggles, and shared directories.
   - `UiAndThemeConfig`: Active skin path, visualizer FPS, docking magnetic threshold, and compact mode flag.
3. Extensible Plugin Schema: Dynamic key-value TOML map (`HashMap<String, toml::Value>`) for third-party plugin options.

## Verification
```bash
cargo test -p amp-domain test_config_schema
```
