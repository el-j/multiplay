# TASK-134: Honest AudioSinkPort: Remove Dead push API, Real configure, Correct Latency

## Plan Reference
- Plan: [PLAN-038](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-038-audio-output-device-robustness.md)
- Role: `architecture`
- Priority: **P2**
- Audit findings: `AUD-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Redesign `AudioSinkPort` around the real data path (producer handle + control), delete `push_interleaved_samples` stub.
2. `configure` actually reopens the stream or is removed; `latency()` computed from frames, rate and device-reported buffer.

## Acceptance Criteria (user-visible / measurable)
- No port method returns constant/fake values.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-ports/src/driven.rs`
- `crates/amp-adapter-cpal/src/lib.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
- Removed dead `push_interleaved_samples` method from `AudioSinkPort` (`crates/amp-ports/src/driven.rs`).
- Implemented real stream reconfiguration in `CpalAudioSink::configure` and accurate latency computation (`(queued_frames + device_buffer_frames) / sample_rate`) in `CpalAudioSink::get_buffer_latency`.
- Verified with `./scripts/verify_pure_rust.sh && cargo test -p amp-adapter-cpal && cargo test -p amp-core-engine --test playback_pipeline`.

## Status
- **Status:** Pending
