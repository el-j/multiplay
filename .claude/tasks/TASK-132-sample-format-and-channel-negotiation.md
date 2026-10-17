# TASK-132: Sample-Format & Channel-Count Negotiation (f32/i16/u16/i32, N channels)

## Plan Reference
- Plan: [PLAN-038](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-038-audio-output-device-robustness.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: `AUD-05`, `AUD-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Choose the best supported config: prefer f32 stereo at the source rate (when `bit_perfect` mode) else device default rate.
2. Generic callback over `cpal::SizedSample` with conversion + TPDF dither for integer formats.
3. Map stereo to the first two of N device channels (silence elsewhere) when stereo isn't offered.

## Acceptance Criteria (user-visible / measurable)
- Manual matrix: built-in speakers, USB DAC (i16/i32), HDMI 8-ch, Bluetooth; Windows WASAPI shared.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-cpal/src/device.rs`
- `crates/amp-adapter-cpal/src/actor.rs`

## Verification Command
`cargo test -p amp-adapter-cpal format`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
