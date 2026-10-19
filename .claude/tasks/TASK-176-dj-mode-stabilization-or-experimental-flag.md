# TASK-176: DJ Mode: Experimental Flag, Own Engine Thread, Per-Deck Resampling

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `audio_engine`
- Priority: **P2**
- Audit findings: `UX-08`, `ORC-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125, TASK-142
- **Superseded by:** PLAN-054 (TASK-202 engine) and PLAN-055 (TASK-209..213 UX); close via Nexus when those land.

## Objectives
1. Hide DJ console behind `experimental.dj_mode`.
2. Move deck decoding to async loaders, per-deck resamplers, no per-tick state cloning; integrate BPM detector or remove its UI.

## Acceptance Criteria (user-visible / measurable)
- DJ mode with two decks at different sample rates plays at correct pitch without underruns.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/dj/`
- `crates/amp-adapter-gui/src/dj_*.rs`

## Verification Command
`cargo test -p amp-core-engine dj`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
