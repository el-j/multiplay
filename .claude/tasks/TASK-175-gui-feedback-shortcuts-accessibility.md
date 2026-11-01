# TASK-175: GUI Feedback, Shortcuts, Seek-on-Release & Accessibility

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-07`, `ARC-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-130

## Objectives
1. Toasts/status bar for `PlayerEvent::Error` and buffering; seek on drag release only; 'Add URL' field.
2. Winamp-style shortcuts (Z X C V B, J, Ctrl+L) + platform shortcuts; font scaling, contrast theme, AccessKit labels.
3. Non-blocking file dialogs (spawn off UI thread or `rfd` with xdg-portal on Linux); cancelling never triggers a scan.

## Acceptance Criteria (user-visible / measurable)
- Keyboard-only usage possible; errors are visible in a .app build.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-047-ux)
- Keyboard: `crates/amp-adapter-gui/src/transport.rs` — one `TransportAction` table for buttons and keys: Z prev, X play, C / space pause-resume, V stop, B next, ←/→ seek 5 s, ↑/↓ volume 5%, J jump (search), Ctrl/Cmd+L open location (search window, cursor in the link field). Inactive while a text field has focus or DJ mode (own key map) is on; Cmd+C/Cmd+V stay copy/paste. Tooltips name the keys.
- Seek on release: `transport::Scrub` — dragging moves the knob only, release seeks once, a click seeks at once.
- Feedback: "⏳ BUFFERING n%" on the faceplate; playback errors were already shown (playback_notice).
- Non-blocking dialogs: `dialog_jobs.rs` runs the add/folder dialogs on a thread, polled per frame; a cancelled dialog adds and scans nothing; a second dialog while one is open is ignored.
- Accessibility: eframe `accesskit` feature on (VoiceOver / Narrator / Orca read egui's widget labels); verify_pure_rust and cargo-deny pass with it.
- Tests: `tests/transport_keys.rs`, `tests/dialog_jobs.rs`.
- Not done: a separate high-contrast theme; the skin dialog still opens on the UI thread; screen-reader check by hand.
