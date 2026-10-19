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
- 2026-10-19: one playlist with deck assignments:
  - `amp_domain::DeckAssignment` (none, A, B, both) with `next_for_deck` and `next_markers`. Each deck takes its next unplayed assigned track in playlist order; a both row goes to whichever deck asks first; unassigned rows are skipped. The markers never point both decks at the same row.
  - In DJ mode every playlist row has a badge: click cycles none, A, B, both, and the keys `[`, `]` and `\` assign the selected row (pressing again clears it). Rows show "▸A" and "▸B" next-on markers, played tracks are dimmed, and a click selects instead of playing.
  - Assignments are saved with the session (`SessionPlaylistSnapshot::deck_assignments`, serde default for old files).
  - The console's load button takes the deck's next assigned track (else the selected row). AUTO runs auto mix through the assigned playlist via `PlaylistDeckSource`, taking over the deck on air (`AutoMix::take_over`) and adopting analyses that finish after a track is loaded.
- Evidence:
  - 5 domain tests (order, both rows, markers, removal, badge cycle);
  - `tests/dj_playlist.rs`: a 6-track set A/B/none/both/A/B is handed out as one, two, four, five, six, and assignments survive a restart;
  - `dj_automix_takes_over_a_playing_deck_and_waits_for_late_analysis` (core): a synced transition after analyses land 2 s late;
  - `responsive_playlist_with_deck_badges_fits` at 300 to 700 px.
- Deviation: dragging a row onto a deck is not done. The playlist and the console are separate OS windows, and egui drag-and-drop does not cross them; the badge, keys and load button cover it.
- Open: the manual six-track auto-mix check in the running app.

## Status
- **Status:** Pending
