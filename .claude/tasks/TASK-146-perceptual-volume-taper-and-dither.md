# TASK-146: Perceptual Volume Taper & TPDF Dither

## Plan Reference
- Plan: [PLAN-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-040-audiophile-dsp-transparency.md)
- Role: `dsp_engineering`
- Priority: **P3**
- Audit findings: `DSP-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-132

## Objectives
1. dB-mapped volume curve (−60 dB … 0 dB); TPDF dither when output is ≤ 16-bit integer; volume smoothing to avoid zipper noise.

## Acceptance Criteria (user-visible / measurable)
- Slider midpoint ≈ −20 dB; no zipper noise on fast volume drags (spectral test).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/dsp/chain.rs`

## Verification Command
`cargo test -p amp-core-engine volume`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
