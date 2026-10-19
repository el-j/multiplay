# TASK-209: DJ Mode Toggle: Main Window Becomes the Deck Console Without a Gap

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: `UX-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-202, TASK-195

## Objectives
1. DJ button / Ctrl+J swaps the main window into the 550x232 console (two deck strips + mixer column); it keeps anchoring the dock group.
2. The playing track continues seamlessly as Deck A; leaving DJ mode keeps the on-air deck playing as normal playback.
3. Replace `dj_panel.rs` / `dj_playback.rs` / `dj_audio.rs` with views over the engine.

## Acceptance Criteria (user-visible / measurable)
- Toggling DJ mode during playback produces no audible gap (capture-sink test across the toggle).
- Docked EQ/playlist reflow to the console width and back.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/dj_panel.rs`
- `crates/amp-adapter-gui/src/main_window.rs`

## Verification Command
`cargo test -p amp-adapter-gui dj_mode`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
