# TASK-173: OS Media Keys & Now Playing (MPRIS / macOS / SMTC)

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. `MediaControlsPort` + `souvlaki` adapter: play/pause/next/prev/seek, metadata + artwork; works in GUI and TUI.

## Acceptance Criteria (user-visible / measurable)
- Hardware media keys and Control Center work on macOS; `playerctl` works on Linux; SMTC overlay on Windows.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-media-controls/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-047-ux-2)
- Port: `amp_ports::media_controls::{MediaControlsPort, NowPlaying, MediaPlayback, MediaKeyEvent}`.
- Adapter: new crate `crates/amp-adapter-media-controls` (`OsMediaControls`) on souvlaki `=0.9.3` with `use_zbus` (pure-Rust D-Bus, not libdbus); `verify_pure_rust.sh` (all targets) and `cargo deny` pass; `cargo check --target x86_64-pc-windows-msvc` passes. Events wake the window through a callback (no polling); playback updates only on state changes and seeks (`PlaybackSync`).
- GUI: `media_session.rs` — Now Playing from the engine, media keys through the shared `TransportAction`s; main window HWND passed on Windows. TUI: MPRIS on Linux only (macOS needs an AppKit run loop, Windows a window).
- Tests: `crates/amp-adapter-media-controls/tests/mapping.rs` (event mapping, update throttling), `crates/amp-adapter-gui/tests/media_keys.rs` (stand-in OS session: Now Playing title/album, playing/paused state, Next and Pause keys).
- docs/privacy.md: what Now Playing shares (locally).
- Not verified by hand yet: hardware keys and Control Center on macOS, `playerctl` on Linux, the Windows overlay.
