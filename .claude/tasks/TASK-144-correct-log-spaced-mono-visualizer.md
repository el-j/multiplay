# TASK-144: Visualizer: Mono Downmix, Log-Spaced Bands, dB Scaling

## Plan Reference
- Plan: [PLAN-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-040-audiophile-dsp-transparency.md)
- Role: `dsp_engineering`
- Priority: **P2**
- Audit findings: `DSP-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-128

## Objectives
1. Downmix to mono before FFT; 16 (and 75 for Winamp-style) log-spaced bands 40 Hz–16 kHz; dB scale with peak hold/decay; DC excluded.

## Acceptance Criteria (user-visible / measurable)
- A 1 kHz sine lights exactly the band containing 1 kHz.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/dsp/visualizer.rs`

## Verification Command
`cargo test -p amp-core-engine visualizer`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
