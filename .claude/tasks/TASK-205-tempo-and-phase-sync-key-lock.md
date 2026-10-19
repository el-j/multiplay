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

## Status
- **Status:** Pending
