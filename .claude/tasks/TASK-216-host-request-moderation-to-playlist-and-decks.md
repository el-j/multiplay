# TASK-216: Host Moderation: Accept to Playlist / Play Next / Deck, Reject, Drag & Trusted Devices

## Plan Reference
- Plan: [PLAN-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-056-remote-and-sharing-window-host-moderated-requests.md)
- Design: [Signature features design §7](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-214, TASK-215, TASK-210

## Objectives
1. Request rows show requester, resolved title/artist/duration preview and source badge.
2. Actions: add to end, play next, assign to deck A/B (DJ mode), reject; drag a row into the playlist or onto a deck.
3. Per-device 'trusted: auto-accept'; playlist entries remember the requester.
4. Accepted links are resolved and streamed by the host's own resolvers.

## Acceptance Criteria (user-visible / measurable)
- Integration test: phone requests a YouTube reference → host accepts → track appears in the playlist resolved by the host resolver (mocked network fixture).
- Rejected requests never touch the playlist.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/remote_window.rs`
- `crates/amp-adapter-gui/src/app_render.rs`

## Verification Command
`cargo test -p amp-adapter-gui moderation`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
