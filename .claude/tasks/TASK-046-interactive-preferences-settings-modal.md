---
id: TASK-046
title: Interactive Preferences Settings Modal Interface for TUI and GUI
planId: PLAN-014
status: planned
role: frontend_ui
verificationCommand: cargo test -p amp-adapter-tui test_settings_view && cargo test -p amp-adapter-gui
---

# TASK-046: Interactive Preferences Settings Modal Interface for TUI and GUI

## Description
Implement the categorized preferences modal view in `crates/amp-adapter-tui/src/settings_view.rs` and `crates/amp-adapter-gui`:
1. `SettingsCategory`:
   - `MusicFolders`: Local folder management, add/remove paths, and trigger on-demand scan.
   - `AudioHardware`: Output device selector, buffer size, RAM precache, and target LUFS level.
   - `OnlineAccounts`: Status of third-party streaming providers with password/token entry modals.
   - `P2PNetwork`: Mesh ports, upload slots, Soulseek configuration.
   - `Appearance`: Visualizer FPS, magnetic snap threshold, active skin selector.
   - `PluginOptions`: Dynamic editor for installed third-party plugins.
2. Dual-Interface Rendering:
   - TUI: Two-pane Ratatui modal with sidebar category navigation and keyboard bindings (`Tab`, `a`, `d`, `r`).
   - GUI: Native egui settings panel with tab navigation and immediate parameter application.

## Verification
```bash
cargo test -p amp-adapter-tui test_settings_view && cargo test -p amp-adapter-gui
```
