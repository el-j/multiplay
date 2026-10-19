# TASK-201: Settings as a One-Time Overlay Window

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-195

## Objectives
1. Settings opens as one centered, non-dockable window above the others (menu button or Ctrl+P).
2. Closes with the close button, Esc or 'Done'; remembers the last tab; never snaps.
3. Move pairing QR and paired-device management to the Remote window (TASK-214); Settings keeps only the remote on/off and LAN switches.

## Acceptance Criteria (user-visible / measurable)
- Opening and closing Settings never moves or undocks other windows.
- Test: Settings window id is excluded from the dock graph.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/settings_panel.rs`
- `crates/amp-adapter-gui/src/viewports.rs`

## Verification Command
`cargo test -p amp-adapter-gui settings`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: Settings opens centered on the screen (over the main window when the screen size is unknown; `chrome::overlay::centered`, 3 tests), each time it is opened. Ctrl+P / Cmd+P opens and closes it from any window; it closes with x, Esc, the new "Done" button, or the shortcut. It is not in the dock graph (`dockable_windows_skip_hidden_and_settings`), so opening and closing never moves other windows. It never reopens by itself after a restart (`layout_never_reopens_the_settings_overlay`). The last tab is kept for the session.
- Open: objective 3 (pairing QR and device management move to the Remote window) waits for TASK-214. Centering is unit-tested only; I did not open the overlay by hand (no input automation in this session).

## Status
- **Status:** Pending
