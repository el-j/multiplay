---
id: TASK-027
title: Classic Winamp Equalizer & Playlist Editor Docked Widgets
planId: PLAN-009
status: completed
role: frontend_tui
verificationCommand: cargo test -p amp-adapter-tui test_equalizer_widget_rendering test_playlist_widget_rendering
---

# TASK-027: Classic Winamp Equalizer & Playlist Editor Docked Widgets

## Description
Implement the docked child window widgets in `crates/amp-adapter-tui`:
1. `EqualizerWidget` in `crates/amp-adapter-tui/src/equalizer_view.rs`:
   - Renders header with `WINAMP EQUALIZER` title and `[ON] [AUTO]` toggle indicators.
   - Preamp gain display `PREAMP [±X.X dB]`.
   - 10 frequency band gain sliders (60Hz, 170Hz, 310Hz, 600Hz, 1kHz, 3kHz, 6kHz, 12kHz, 14kHz, 16kHz) with visual directional glyphs (`▲`, `──`, `▼`).
2. `PlaylistWidget` in `crates/amp-adapter-tui/src/playlist_view.rs`:
   - Classic Winamp pledit interface rendering track numbers, artists, titles, and formatted durations.
   - Active cursor highlight row (`▶`) and bottom status bar displaying total tracks and total duration `MM:SS`.
3. Integrate into `crates/amp-app/src/main.rs`:
   - Wire `Up` and `Down` arrow keys for playlist cursor navigation.
   - Wire `Enter` to immediately select and trigger playback.
   - Wire `e` and `p` to toggle Equalizer and Playlist window docking visibility.

## Verification
```bash
cargo test -p amp-adapter-tui test_equalizer_widget_rendering test_playlist_widget_rendering
```
