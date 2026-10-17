# TASK-141: Lookahead, Oversampled True-Peak Limiter

## Plan Reference
- Plan: [PLAN-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-040-audiophile-dsp-transparency.md)
- Role: `dsp_engineering`
- Priority: **P2**
- Audit findings: `DSP-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-140

## Objectives
1. Lookahead (≈2 ms) limiter with 4× oversampled peak detection, smooth attack/release envelope; zero allocations in process().

## Acceptance Criteria (user-visible / measurable)
- Sine sweeps at +6 dB gain stay below −1 dBTP with THD < 0.1% below threshold.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/dsp/limiter.rs`

## Verification Command
`cargo test -p amp-core-engine limiter`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
