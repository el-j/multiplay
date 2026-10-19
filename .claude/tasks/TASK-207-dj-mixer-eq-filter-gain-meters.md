# TASK-207: DJ Mixer: Trim/Auto-Gain, 3-Band EQ & Kills, Filter, Faders, Crossfader, Limiter, Meters

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `dsp_engineering`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-202, TASK-143

## Objectives
1. Per deck: trim with optional loudness auto-gain, 3-band EQ with kills, one-knob LP/HP filter, channel fader; crossfader with curve choice; master limiter.
2. Per-deck and master peak/RMS meters exposed through a lock-free tap.
3. Classic EQ window acts on the master bus in DJ mode.

## Acceptance Criteria (user-visible / measurable)
- Unit tests: EQ/filter responses within ±0.5 dB of design; equal-power crossfader at centre; limiter prevents overs on a +6 dB test mix.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/mixer.rs`
- `crates/amp-core-engine/src/dj/deck.rs`

## Verification Command
`cargo test -p amp-core-engine dj_mixer`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
