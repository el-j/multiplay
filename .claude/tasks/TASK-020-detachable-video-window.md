---
id: TASK-020
title: Detachable Video Stream Canvas View
planId: PLAN-006
status: completed
role: frontend_tui
verificationCommand: cargo test -p amp-adapter-tui test_video_view
---

# TASK-020: Detachable Video Stream Canvas View

## Description
Implement `VideoWindowView` in `crates/amp-adapter-tui/src/video_view.rs`:
- Implements `ratatui::widgets::Widget` for detachable video rendering.
- Can be torn off into a floating window (`Alt + W` or `v`).
- When playing video sources (YouTube, Bandcamp video), renders 24-bit truecolor half-block Unicode pixels (`▀`) into the terminal cell buffer with zero heap allocations during render loop.
- Displays audio-only placeholder when no video track is active.

## Verification
```bash
cargo test -p amp-adapter-tui test_video_view
```
