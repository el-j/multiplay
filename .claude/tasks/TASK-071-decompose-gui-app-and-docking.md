---
id: TASK-071
title: Decompose GUI App and Docking Modules into Focused Subsystems
planId: PLAN-023
status: pending
role: gui_engineering
verificationCommand: cargo check -p amp-adapter-gui
---

# TASK-071: Decompose GUI App and Docking Modules into Focused Subsystems

## Description
Refactor the large GUI modules in `amp-adapter-gui` (`app.rs` at 447 lines and `docking.rs` at 349 lines):
1. `docking/geometry.rs`: Rectangular bounding box math, distance calculations, and overlap detection.
2. `docking/snapping.rs`: Magnetic edge snapping algorithm ($\le 14$px) and group translation calculation.
3. `docking/layout_storage.rs`: Window position persistence to `~/.config/multiplay/window_layout.json`.
4. `app/state.rs`: GUI state models, active tabs, modals, and settings cache.
5. `app/event_dispatcher.rs`: Keyboard shortcut routing, file drag-and-drop forwarding, and engine event processing.

## Verification
```bash
cargo check -p amp-adapter-gui
```

## Acceptance Criteria
- No file in `crates/amp-adapter-gui/src/` exceeds 300 lines.
- Complete modular separation of math, layout persistence, and UI rendering.
- Zero visual or behavioral regressions in the desktop GUI application.
