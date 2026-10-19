# TASK-210: One Playlist, Deck Assignment (A / B / Both) & 'Next on Deck' Markers

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-209

## Objectives
1. In DJ mode each row shows a deck badge: A, B, A+B or none; assign by badge click, keys `[`, `]`, `\`, or drag onto a deck.
2. Each deck plays its assigned tracks in playlist order; rows marked 'next on A/B'; unassigned rows skipped in DJ mode.
3. Assignment stored with the session playlist.

## Acceptance Criteria (user-visible / measurable)
- Unit tests for next-track selection per deck including A+B rows and removals.
- Manual: build a 6-track set, assign alternately, auto mix plays it in order.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/playlist_window.rs`
- `crates/amp-domain/src/`

## Verification Command
`cargo test -p amp-adapter-gui deck_assign`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
