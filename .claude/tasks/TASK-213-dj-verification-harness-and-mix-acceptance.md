# TASK-213: DJ Verification Harness & Live-Mix Acceptance

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `qa_and_hardening`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-208, TASK-210, TASK-211, TASK-212

## Objectives
1. End-to-end tests: sync + crossfade of 120/124 BPM fixtures, cue routing to capture sinks, auto mix through an assigned playlist, DJ toggle mid-track.
2. 10-minute soak with continuous parameter changes: zero underruns, no allocations in the callback.
3. Remove `experimental.dj_mode` gate when all pass; update README feature matrix.

## Acceptance Criteria (user-visible / measurable)
- All DJ tests green on macOS, Linux and Windows CI.
- Release-build manual mix (two decks, prelisten, loop, auto mix) recorded as evidence.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/tests/dj_*.rs`
- `crates/amp-app/tests/dj_engine_pipeline.rs`
- `README.md`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace dj`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
