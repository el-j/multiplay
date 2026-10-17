# TASK-125: PlayerEngine: State Machine Implementing PlayerControlPort & PlaylistManagementPort

## Plan Reference
- Plan: [PLAN-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-037-unified-core-playback-engine.md)
- Role: `audio_engine`
- Priority: **P0**
- Audit findings: `ARC-01`, `ARC-02`, `ARC-05`, `AUD-14` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Design doc (ADR) for the engine: threads (control task, decode worker, RT callback), channels, state machine `Stopped → Loading → Buffering → Playing ⇄ Paused → Stopped | Error`.
2. Implement `PlayerEngine` in `crates/amp-core-engine/src/player/` consuming `PlayerCommand` and emitting `PlayerEvent`; implements `PlayerControlPort` and `PlaylistManagementPort`.
3. Sink, decoder factory and resolvers are injected through ports (no concrete adapters in core).
4. Event-driven worker: parks on ring space/commands; clean shutdown on drop (join threads).

## Acceptance Criteria (user-visible / measurable)
- Engine passes a port contract test suite using `CaptureAudioSink` and in-memory WAV fixtures: play, pause, resume, stop, seek, next, end-of-queue.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/player/`
- `crates/amp-ports/src/driving.rs`
- `docs/adr/ADR-001-player-engine.md`

## Verification Command
`cargo test -p amp-core-engine player`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
