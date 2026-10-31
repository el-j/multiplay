# TASK-128: Output-Clock Playback Position & Output-Aligned Visualizer Tap

## Plan Reference
- Plan: [PLAN-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-037-unified-core-playback-engine.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: `AUD-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-127

## Objectives
1. RT callback increments `AtomicU64` frames-consumed (tagged by epoch); position = seek base + consumed frames converted at device rate.
2. Seek sets base from the decoder's actual seeked timestamp.
3. Visualizer/peak meters read from a lock-free tap filled by the callback (or delayed by measured latency).

## Acceptance Criteria (user-visible / measurable)
- Displayed position within ±20 ms of audio heard (CaptureSink test), including after seeks.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-cpal/src/actor.rs`
- `crates/amp-core-engine/src/player/`

## Verification Command
`cargo test -p amp-core-engine position`

## Manual Verification
- Implemented speaker-aligned playback position (`seek_base_micros` + consumed frames at device rate) and lock-free 2048-sample `AtomicU32` visualizer tap in `crates/amp-ports/src/clock.rs`.
- Verified with `cargo test -p amp-core-engine position` (`position_tracks_speaker_output_clock_within_20ms_and_updates_tap` passed) and `cargo test -p amp-ports`.

## Status
- **Status:** Pending
