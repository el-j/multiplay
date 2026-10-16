---
id: TASK-052
title: Modular Windows Implementation (Player, Equalizer, Playlist, Settings, DJ Console)
planId: PLAN-016
status: completed
role: frontend_gui
verificationCommand: cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui
---

# TASK-052: Modular Windows Implementation (Player, Equalizer, Playlist, Settings, DJ Console)

## Description
Decompose the monolithic `CentralPanel` GUI into distinct, independent `egui::Window` structures styled with the retro Winamp dark titanium aesthetic:
1. `MainWindow`: Contains title bar, glowing VFD track info & time counter, audio output device selector, 16-band visualizer canvas, transport controls, and volume slider.
2. `EqualizerWindow`: Contains preamp slider, 10-band IIR graphic EQ faders, EQ toggle switch, reset button, and preset buttons.
3. `PlaylistWindow`: Contains track list with source badges, search bar, duration summary, drag-and-drop ingestion zone, and playback control.
4. `SettingsWindow`: Multi-tab preference dialog (Audio Hardware, Paths & Storage, Secrets Vault, Visualizer / Theme).
5. `DjConsoleWindow`: Dual-deck DJ controls, crossfader, pitch sliders, and cue toggles.
6. Each window supports independent resizing, moving, minimize/shade mode, and toggle visibility.

## Verification
```bash
cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui
```
