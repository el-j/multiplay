---
id: TASK-053
title: Window Layout Persistence & Session Restore
planId: PLAN-016
status: completed
role: frontend_gui
verificationCommand: cargo test -p amp-adapter-gui test_layout_persistence
---

# TASK-053: Window Layout Persistence & Session Restore

## Description
Persist modular desktop window positions, dimensions, visibility, and docking states across application sessions:
1. Extend `AppConfig` in `amp-domain` with a `WindowLayoutConfig` schema storing `x`, `y`, `width`, `height`, `is_open`, `is_shaded`, and `docked_to`.
2. Save layout configuration to disk (`~/.config/multiplay/config.toml` or `config.json`) upon graceful shutdown or window move/resize end.
3. On application startup, restore window bounds and docked group relationships.
4. Screen Clamping & Multi-Monitor Safety: Implement bounds checking to ensure restored windows never appear offscreen if the display resolution or monitor setup changes.

## Verification
```bash
cargo test -p amp-adapter-gui test_layout_persistence
```
