---
id: TASK-042
title: Multi-Deck Hardware-Style DJ Console Interface for TUI and GUI
planId: PLAN-013
status: completed
role: frontend_ui
verificationCommand: cargo test -p amp-adapter-tui test_dj_console && cargo test -p amp-adapter-gui
---

# TASK-042: Multi-Deck Hardware-Style DJ Console Interface for TUI and GUI

## Description
Implement the professional multi-deck hardware DJ console layout across both TUI (`ratatui`) and native desktop GUI (`egui`):
1. Deck Panels: Side-by-side Deck A and Deck B panels displaying:
   - Track titles, detected BPM, Camelot musical key (e.g., `4A`), and pitch slider offset (`-8%` to `+8%`).
   - Scrolling waveform progress bar with moving beat markers and hot cue points (1, 2, 3, 4).
   - Kill EQ indicators (`HI`, `MID`, `LOW`) and volume fader levels.
2. Central Mixer Section:
   - Interactive horizontal crossfader with curve selector (`SMOOTH` vs `SHARP CUT`).
   - Headphone Cue (PFL) toggle buttons for Deck A and Deck B.
   - Master Tempo (Key Lock) toggle button.

## Verification
```bash
cargo test -p amp-adapter-tui test_dj_console && cargo test -p amp-adapter-gui
```
