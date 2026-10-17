# TASK-142: Streaming-Correct Resampler (No Mid-Stream Padding, Per-Stream Instances)

## Plan Reference
- Plan: [PLAN-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-040-audiophile-dsp-transparency.md)
- Role: `dsp_engineering`
- Priority: **P1**
- Audit findings: `DSP-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Input FIFO feeding rubato only full blocks; `process_partial` only at true EOF; compensate output delay.
2. One resampler per stream (main, preload/crossfade, each DJ deck); pre-allocated buffers.

## Acceptance Criteria (user-visible / measurable)
- Resampling a 44.1 k sine to 48 k through random-size input chunks equals the single-shot result (SNR > 120 dB); no zero runs.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/dsp/resampler.rs`

## Verification Command
`cargo test -p amp-core-engine resampler`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
