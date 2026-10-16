---
id: PLAN-006
title: Dense Winamp 1.x Faceplate & Multi-Window Magnetic Docking TUI
status: completed
role: frontend_tui
tasks:
  - TASK-017
  - TASK-018
  - TASK-019
  - TASK-020
---

# PLAN-006: Dense Winamp 1.x Faceplate & Multi-Window Magnetic Docking TUI

## Objective
Construct the pixel-dense Winamp 1.x terminal interface using `ratatui`: tape-deck faceplate with digital VFD time display, mini-spectrum analyzer, volume/balance gauges, magnetic multi-window docking manager with snapping and tear-off views, omnibar search dialog with source badges, and detachable video mode canvas.

## Architecture Context
- Vision Docs: `docs/vision/dense_winamp_1_faceplate_multi_window_docking.md`, `docs/vision/multi_source_search_filters_source_badges.md`.
- Target Crate:
  - `crates/amp-adapter-tui`:
    - `dense_console.rs`: Compact $78 \times 7$ cell console with LED time display, mini spectrum analyzer, volume meter, and transport buttons.
    - `docking.rs`: `DockingWindowManager` with `WindowSlot`, edge snap calculation (`SNAP_THRESHOLD_CELLS`), and tear-off toggle.
    - `search_view.rs`: Omnibar modal with source filter toggle pills and badge-attributed result rows.
    - `video_view.rs`: Detachable video player rendering half-block Unicode truecolor video frames.

## Tasks
1. [TASK-017](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-017-dense-winamp-faceplate.md): Dense Winamp 1.x Faceplate Terminal Widget (`ratatui`)
2. [TASK-018](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-018-magnetic-docking-window-manager.md): Magnetic Multi-Window Docking & Snapping Window Manager
3. [TASK-019](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-019-search-modal-source-badges.md): Omnibar Search Modal with Filter Pills & Source Badges
4. [TASK-020](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-020-detachable-video-window.md): Detachable Video Stream Canvas View

## Definition of Done
- TUI faceplate renders crisply at $\ge 78 \times 14$ terminal dimensions with zero flicker.
- Magnetic snapping locks windows together when edges approach within 2 cells.
- Search modal allows toggling source filters dynamically using `Tab` and triggers downloads via `d`.
