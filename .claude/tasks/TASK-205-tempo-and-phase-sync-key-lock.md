# TASK-205: Beat Matching: Tempo & Phase Sync, Quantize, Pitch Range, Key Lock, Nudge

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `dsp_engineering`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-204

## Objectives
1. Sync tempo to the master deck and align beat phase; quantize for cue/loop actions.
2. Pitch ranges ±8/16/50 %; key lock (tempo change without pitch change) via the time-stretcher; pitch-bend nudge.
3. Master-deck selection (auto: the louder/on-air deck).

## Acceptance Criteria (user-visible / measurable)
- Synced decks stay within 5 ms beat phase over 5 minutes (capture-sink test, synthetic 120 vs 124 BPM tracks).
- Key lock at ±8 % keeps the fundamental within ±5 cents.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/sync.rs (new)`
- `crates/amp-core-engine/src/dj/wsola.rs`

## Verification Command
`cargo test -p amp-core-engine dj_sync`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: beat matching in the engine (`dj::engine::sync`, `keylock`, ADR-003 §8):
  - **Master and sync.** Master deck is explicit or auto (loudest playing deck with a grid). A synced deck follows the master's heard tempo, with half/double time, and steers phase errors out within about 0.25 s at no more than 3 %.
  - **Snapping.** On play and on seek, a synced deck lands on the beat at once, moving forward when moving back would pass the track start.
  - **Pitch.** Ranges ±8/16/50 % (`PitchRange`); nudge (±10 %) while held.
  - **Key lock.** WSOLA on the playhead, so tempo changes without pitch change.
  - **Grids and status.** Grids come from TASK-204 analysis via `DeckParams::set_grid`; status reports each deck's heard BPM and master flag.
  - **Quantize.** Snap times with `TrackAnalysis::nearest_beat`; the engine-side quantize of loops and cues comes with TASK-206.
- Evidence (`cargo test -p amp-core-engine dj`, checked on rendered audio):
  - a 124 BPM deck synced to a 120 BPM master (one on the cue bus, one on master) keeps every beat within 5 ms over 5 minutes;
  - a synced deck starts in phase within 2 ms;
  - after a seek it lands on the beat within 5 ms;
  - after the master jumps a fifth of a beat, followers steer back within 5 ms;
  - key lock at +8 % and -8 % keeps 440 Hz within 5 cents while playing 8 % faster or slower, and turning it off moves the pitch as expected;
  - nudge bends the tempo exactly;
  - render stays allocation-free with sync and key lock on.
- Mutations: with steering disabled the master-jump test fails. Two of my own test bugs were found on the way: an onset detector that found nothing, which let a test pass vacuously, and a block-count rounding error. Both are fixed, and the gap helper now fails on empty input.
- Open: grids in these tests are given exactly, so analysis accuracy is tested separately (TASK-204). Key lock quality on real music needs a listening check once the UI is wired (PLAN-055).

## Status
- **Status:** Pending
