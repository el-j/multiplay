# TASK-214: Remote & Sharing Window (Dockable, Playlist-Style): Requests, Devices, Shares

## Plan Reference
- Plan: [PLAN-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-056-remote-and-sharing-window-host-moderated-requests.md)
- Design: [Signature features design §7](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-197, TASK-198, TASK-117

## Objectives
1. New dockable, resizable window with Requests, Devices and Shares sections.
2. Devices: paired phones with name and last seen, revoke one or all, pairing QR and 'new pairing code' (moved from Settings).
3. Opened from the main window (button next to EQ/PL) and remembered in the layout.

## Acceptance Criteria (user-visible / measurable)
- Window docks, shades and resizes like the playlist.
- Revoking one device closes only its session (integration test).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/remote_window.rs (new)`
- `crates/amp-adapter-gui/src/remote_manager.rs`
- `crates/amp-adapter-remote/src/auth.rs`

## Verification Command
`cargo test -p amp-adapter-gui remote_window`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
