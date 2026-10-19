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
