---
id: PLAN-007
title: Classic Winamp Skin (.wsz) Loader & Pure-Rust Plugin System
status: planned
role: extensibility
tasks:
  - TASK-021
  - TASK-022
---

# PLAN-007: Classic Winamp Skin (.wsz) Loader & Pure-Rust Plugin System

## Objective
Implement pure-Rust decompression and parsing of classic Winamp `.wsz` skin packages (ZIP archives containing `viscolor.txt`, `pledit.txt`, and bitmap sheets) to skin the TUI/GUI, along with a safe pure-Rust plugin registry and capability manager without unsafe native dynamic library loading.

## Architecture Context
- Vision Docs: `docs/vision/pure_rust_plugin_skin_architecture.md`.
- Target Modules:
  - `crates/amp-core-engine/src/skin.rs`: `SkinLoader` parsing `.wsz` zip archives, extracting RGB values from `viscolor.txt` (16-color spectrum gradient) and `pledit.txt` (`NormalBG`, `Normal`).
  - `crates/amp-ports/src/plugin.rs`: `AmpAudioPlugin`, `PluginCapability`, and `DspFilterHook` traits.
  - `crates/amp-core-engine/src/plugin_registry.rs`: `PluginRegistry` coordinating in-tree extensions and WASM sandbox modules.

## Tasks
1. [TASK-021](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-021-winamp-wsz-skin-loader.md): Winamp Classic Skin (.wsz) Decompressor & Palette Extractor
2. [TASK-022](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-022-pure-rust-plugin-host.md): Safe Pure-Rust Plugin Host & Capability Registry

## Definition of Done
- `SkinLoader::load_from_wsz_bytes` successfully parses classic skin archives into `WinampTheme`.
- `PluginRegistry` permits compile-time and sandboxed plugin capability queries without `libloading` or unsafe pointers.
