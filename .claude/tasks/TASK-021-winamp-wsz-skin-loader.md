---
id: TASK-021
title: Winamp Classic Skin (.wsz) Decompressor & Palette Extractor
planId: PLAN-007
status: pending
role: extensibility
verificationCommand: cargo test -p amp-core-engine test_skin_loader
---

# TASK-021: Winamp Classic Skin (.wsz) Decompressor & Palette Extractor

## Description
Implement `SkinLoader` in `crates/amp-core-engine/src/skin.rs`:
- Reads `.wsz` archives using pure-Rust `zip` in memory without writing temporary files to disk.
- Parses `viscolor.txt` into a 16-color RGB gradient for spectrum visualizers.
- Parses `pledit.txt` to extract `NormalBG` and `Normal` foreground text colors.
- Generates `WinampTheme` struct containing background, foreground, accent, VFD color, and spectrum gradients for immediate skinning of the Ratatui TUI.

## Verification
```bash
cargo test -p amp-core-engine test_skin_loader
```
