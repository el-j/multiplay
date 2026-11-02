# TASK-220: Four-Deck Layout & Deck Count Setting

## Plan Reference
- Plan: [PLAN-057](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-057-remix-decks-stems-and-mix-recording.md)
- Design: [Signature features design §8](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P2**
- Audit findings: —
- Depends on: TASK-213

## Objectives
1. Deck count setting 2/4; console layout for four decks; assignment badges C/D.
2. Engine already supports 4 decks (TASK-202); add routing/crossfader assignment per deck.

## Acceptance Criteria (user-visible / measurable)
- Four decks mix with zero underruns in a 10-minute soak.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/dj_panel.rs`
- `crates/amp-core-engine/src/dj/`

## Verification Command
`cargo test -p amp-core-engine dj_four_deck`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending

## Evidence (2026-11-02, branch feat/plan-057-dj)
- Setting `audio.dj_deck_count` (2 default, 4), Preferences → Audio "DJ decks". The GUI engine always runs four decks; the console shows two or four, and hiding C/D stops them.
- Console: four detail lanes; strips A/C left, B/D right; mixer faders for every deck with an L/R/T crossfader-side badge (click cycles); minimum console 560×440 with four decks.
- `cargo test -p amp-core-engine dj_four_deck`: assignment routing (A left, B right, C/D through, re-assignable); release soak (10 min, four decks at 44.1/48/22.05/96 kHz, moving crossfader and pitch): 0 underruns, added to CI's DJ release step.
- `crates/amp-adapter-gui/tests/dj_four_deck.rs` (setting, on-air follows assignment), `responsive_layout.rs` four-deck fit at 560×440, 800×500, 1200×700.
- Full gate: 640 passed. Not yet: keyboard shortcuts for C/D (mouse only), release-build manual demo.
