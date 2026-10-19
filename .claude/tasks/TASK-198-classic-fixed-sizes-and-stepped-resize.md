# TASK-198: Classic Fixed Sizes & Stepped Resize Grip (Playlist, Remote)

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-195

## Objectives
1. Main and EQ fixed at 275x116 (1x); playlist and remote windows have a bottom-right grip resizing in 25x29 steps with a 275x116 minimum.
2. Layout of faceplate content adapts to steps (playlist rows, scrollbar) without clipping.
3. HiDPI: logical sizes stay classic on 1x/2x displays; text stays crisp.

## Acceptance Criteria (user-visible / measurable)
- No dockable window can be resized except via the grip; sizes are always on the step grid.
- Unit tests for step snapping of resize deltas.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/playlist_window.rs`
- `crates/amp-adapter-gui/src/chrome/resize.rs (new)`

## Verification Command
`cargo test -p amp-adapter-gui resize`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
