# TASK-133: Device Hot-Plug, Default-Device Change & Stream Error Recovery

## Plan Reference
- Plan: [PLAN-038](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-038-audio-output-device-robustness.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: `AUD-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-132

## Objectives
1. Error callback signals the actor via atomic flag; actor rebuilds the stream on the same ring, flushing by epoch.
2. Follow-system-default mode: poll/subscribe default device and migrate.
3. Emit `PlayerEvent::OutputDeviceChanged`/`Error`; GUI device picker uses the engine.

## Acceptance Criteria (user-visible / measurable)
- Unplugging headphones mid-track resumes on speakers within 1 s without restart.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-cpal/src/actor.rs`

## Verification Command
`manual: plug/unplug test + cargo test -p amp-adapter-cpal recovery`

## Manual Verification
- Implemented stream error callback flag (`stream_error: Arc<AtomicBool>`), follow-system-default polling, epoch ring-buffer flush, and stream rebuild on the shared `Consumer<f32>` in `crates/amp-adapter-cpal/src/actor.rs`.
- Verified with `cargo test -p amp-adapter-cpal recovery` (`recovery_on_stream_error_flushes_epoch_rebuilds_stream_and_migrates_to_default` passed).

## Status
- **Status:** Pending
