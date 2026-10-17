# TASK-184: Real-Audio Fixture Corpus & Golden Decode Tests

## Plan Reference
- Plan: [PLAN-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-050-release-grade-quality-assurance.md)
- Role: `qa_and_hardening`
- Priority: **P1**
- Audit findings: `QA-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Generate CC0 fixtures with known content (sweeps, clicks at known samples): FLAC 16/44.1 & 24/96, MP3 CBR/VBR with LAME gapless info, AAC/M4A with iTunSMPB, Vorbis chained, 5.1 FLAC, truncated/corrupt files.
2. Golden tests: gapless boundary continuity (sample-exact), seek accuracy, duration accuracy, transparent-path null test, corrupt-file graceful errors.

## Acceptance Criteria (user-visible / measurable)
- Fixture suite < 10 MB, runs in < 60 s in CI.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `fixtures/audio/`
- `crates/amp-core-engine/tests/golden_*.rs`
- `scripts/gen_fixtures.rs`

## Verification Command
`cargo test --workspace golden`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
