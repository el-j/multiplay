---
id: TASK-018
title: Magnetic Multi-Window Docking & Snapping Window Manager
planId: PLAN-006
status: completed
role: frontend_tui
verificationCommand: cargo test -p amp-adapter-tui test_docking
---

# TASK-018: Magnetic Multi-Window Docking & Snapping Window Manager

## Description
Implement `DockingWindowManager` in `crates/amp-adapter-tui/src/docking.rs`:
- Window identifiers: `MainConsole`, `Equalizer`, `Playlist`, `LibraryBrowser`, `VideoPlayer`, `SpectrumVisualizer`.
- Stores `WindowSlot`: `Rect`, visibility, docked status (`docked_to`), and z-index.
- Magnetic edge snapping: `move_window_with_magnetic_snap` automatically tests bottom-to-top and left-to-right boundaries using `SNAP_THRESHOLD_CELLS = 2`.
- Tear-off mechanics: `toggle_tear_off` allows detaching or re-docking windows on demand.
- Default classic layout arranging Main, EQ, and Playlist in a locked vertical stack with side Library.

## Verification
```bash
cargo test -p amp-adapter-tui test_docking
```
