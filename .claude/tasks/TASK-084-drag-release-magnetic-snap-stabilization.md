# TASK-084: Drag-Release Magnetic Snap Stabilization and WindowServer Decoupling

## Plan Reference
- Plan: [PLAN-027](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-027-window-stability-scaler-smoothness-and-multiviewport-polish.md)
- Role: `gui_engineering`

## Objectives
1. Introduce `was_dragging: HashSet<WindowId>` state in `MultiplayGuiApp` to track dragging lifecycles.
2. In all window viewports, suppress magnetic snap repositioning while `i.pointer.any_down()` is true.
3. On mouse release (`!i.pointer.any_down()`), compute magnetic snap once against docked targets, updating position cleanly without fighting the macOS WindowServer.

## Verification Command
`cargo test -p amp-adapter-gui test_docking`

## Status
- **Status:** COMPLETED
- **Outcome:** Added `was_dragging` tracking in `handle_satellite_geometry_and_snap`. Repositioning is completely suppressed while pointer is down and executed cleanly on mouse release. Verified with `cargo test -p amp-adapter-gui test_docking` passing without errors.

