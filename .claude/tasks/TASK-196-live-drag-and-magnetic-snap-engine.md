# TASK-196: Live App-Driven Drag & Magnetic Snap Engine (Windows + Screen Edges)

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-195

## Objectives
1. Pure geometry module (no egui types) extending `docking/`: given a moving rect, other visible rects and monitor work areas, return the snapped rect.
2. Snap within `snap_distance` (default 10 px, scaled by double-size) to window edges and work-area edges; align left/right (or top/bottom) edges on contact.
3. Title-bar drag driven by the app in screen space: press records pointer and window origin; each frame sends `OuterPosition(start + delta)` after snapping. No OS drag (`StartDrag`).
4. Replace the release-time snap in `handle_satellite_geometry_and_snap`.

## Acceptance Criteria (user-visible / measurable)
- Dragging the playlist toward the main window snaps flush while the button is still held; release leaves it docked with zero gap.
- Property tests: snapped rect never overlaps another window; distance > threshold never snaps.
- Drag stays smooth (no jitter/oscillation) at 60 Hz on macOS and Windows (screen recording as evidence).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/docking/geometry.rs`
- `crates/amp-adapter-gui/src/docking/snap.rs (new)`
- `crates/amp-adapter-gui/src/viewports.rs`

## Verification Command
`cargo test -p amp-adapter-gui docking`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
