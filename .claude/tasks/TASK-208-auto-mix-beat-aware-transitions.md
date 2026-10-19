# TASK-208: Auto Mix: Automatic Beat-Aware Transitions Between Decks

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-205, TASK-207, TASK-108

## Objectives
1. When on, transition before track end at a configurable point (beats, seconds, or detected outro).
2. Compatible BPMs: sync and beat-aligned transition; otherwise equal-power crossfade.
3. Then load the next assigned track for the freed deck; with none assigned, take the next unplayed playlist track; with nothing left, let the last track end.

## Acceptance Criteria (user-visible / measurable)
- Engine test: a 4-track assigned playlist auto-mixes end to end with no silence and no beat-phase error > 10 ms in synced transitions.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/automix.rs (new)`
- `crates/amp-core-engine/src/dsp/crossfade.rs`

## Verification Command
`cargo test -p amp-core-engine dj_automix`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
