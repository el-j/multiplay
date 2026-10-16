---
id: TASK-017
title: Dense Winamp 1.x Faceplate Terminal Widget
planId: PLAN-006
status: completed
role: frontend_tui
verificationCommand: cargo test -p amp-adapter-tui test_dense_console
---

# TASK-017: Dense Winamp 1.x Faceplate Terminal Widget

## Description
Implement `DenseWinampConsole` in `crates/amp-adapter-tui/src/dense_console.rs`:
- Implements `ratatui::widgets::Widget`.
- Ultra-dense $78 \times 7$ cell faceplate geometry.
- Green vacuum-fluorescent display (VFD) timer rendering track number, play/pause indicator, and elapsed time `MM:SS`.
- 16-bar mini tape-head spectrum visualizer using Unicode block glyphs (` `, `▂`, `▄`, `▆`, `█`).
- Volume and balance level gauges.
- Tape-deck style transport control buttons (`[PREV]`, `[PLAY]`, `[PAUSE]`, `[STOP]`, `[NEXT]`, `[EJECT]`).
- Precaching buffer fill percentage indicator and audio format badge (`[192k][44.1][ST]`).

## Verification
```bash
cargo test -p amp-adapter-tui test_dense_console
```
