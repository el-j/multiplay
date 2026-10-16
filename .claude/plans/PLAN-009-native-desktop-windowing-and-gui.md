---
id: PLAN-009
title: Native OS Desktop Windowing, Equalizer & Playlist Widgets, and Standalone Launcher
status: completed
role: frontend_gui
tasks:
  - TASK-026
  - TASK-027
  - TASK-028
---

# PLAN-009: Native OS Desktop Windowing, Equalizer & Playlist Widgets, and Standalone Launcher

## Objective
Provide standalone native OS desktop windowing capabilities for Multiplay so the application opens its own dedicated native window rather than only running in the invoking terminal session. Expand the modular docking interface with full Equalizer and Playlist Editor widgets, and architect the pure-Rust GUI desktop node (`crates/amp-adapter-gui`).

## Architecture Context
- Vision Docs: `docs/vision/system_architecture_topology.md`, `docs/vision/dense_winamp_1_faceplate_multi_window_docking.md`.
- Target Crates & Tools:
  - `scripts/launch_desktop_window.sh`: Dedicated native floating OS window launcher for macOS with retro geometry ($80 \times 30$).
  - `Multiplay.command` & `Multiplay.app`: Native macOS double-clickable command and Application bundle.
  - `crates/amp-adapter-tui`:
    - `equalizer_view.rs`: `EqualizerWidget` rendering 10-band sliders, preamp gain, and bypass switches into the docked Equalizer window.
    - `playlist_view.rs`: `PlaylistWidget` rendering tracklist rows, active selection cursor, durations, and pledit controls.
  - `crates/amp-app`: Integrated keyboard navigation for playlist selection (`Up`/`Down`/`Enter`) and window toggles (`e`, `p`).
  - `crates/amp-adapter-gui`: Pure-Rust standalone desktop GUI node utilizing hexagonal driving ports.

## Tasks
1. [TASK-026](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-026-native-desktop-window-launcher-and-app-bundle.md): Standalone Native Desktop Window Launcher, macOS Application Bundle & Scripts
2. [TASK-027](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-027-docked-equalizer-and-playlist-editor-widgets.md): Classic Winamp Equalizer & Playlist Editor Docked Widgets
3. [TASK-028](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-028-pure-rust-gui-desktop-node-adapter.md): Pure-Rust GUI Desktop Node Adapter (`crates/amp-adapter-gui`) Architecture

## Definition of Done
- Player can be launched into its own native desktop window from Terminal, Finder, or double-click.
- Docked Equalizer and Playlist windows render their respective controls and tracklists when toggled.
- Pure Rust verification (`./scripts/verify_pure_rust.sh`) remains 100% compliant with zero C dependencies.
