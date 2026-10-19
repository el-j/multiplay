# TASK-199: Persist & Restore Window Layout, Docking and Visibility

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-197, TASK-150

## Objectives
1. Persist positions, dock graph, shade, visibility, playlist size and double-size through the config store (TASK-150); replaces the ad-hoc layout file in `layout_manager.rs`.
2. On start, clamp layouts outside current monitors back on screen with the group intact.
3. Versioned schema with migration from the current layout file.

## Acceptance Criteria (user-visible / measurable)
- Quit and relaunch restores the exact layout on macOS, Windows and X11.
- Test: layout saved on a disconnected second monitor reopens fully visible on the primary.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/layout_manager.rs`
- `crates/amp-domain/src/config.rs`

## Verification Command
`cargo test -p amp-adapter-gui layout`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: versioned layout file (`layout_manager::LayoutSnapshot`, version 1): positions and sizes stored at 1x, plus visibility, shade with restore height, and zoom (double size). Atomic write (temp file, then rename). It is saved whenever the layout changes and no drag, resize or zoom change is under way, and on exit. Before this the layout was loaded but never saved in normal use. The old `DockManager` dump migrates; files from a newer version or garbage are ignored. Docks are rebuilt from touching edges.
- 2026-10-19: `fit_on_screen`: groups whose top-left is on no screen move onto the first screen as one piece (docks kept, no landing on other groups); groups already on a screen stay. 7 tests incl. "unplugged second monitor" and an app restart test (`layout_survives_a_restart`). Verified on macOS with a scratch HOME: file written on first launch, relaunch restored a 2x side-docked layout exactly.
- Deviation (my call): stored in its own file next to the config, not inside the config store (TASK-150 is still pending and window geometry is UI state, not a preference). Open: monitor origins. egui reports only the current monitor's size, so a layout on a secondary monitor left of or above the primary is pulled back to the primary on launch (TASK-200).

## Status
- **Status:** Pending
