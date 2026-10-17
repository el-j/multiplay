# PLAN-038: Audio Output Device Robustness

## Plan Overview
- **ID:** `PLAN-038`
- **Title:** Audio Output Device Robustness
- **Role:** `audio_engine`
- **Milestone:** M1 — It plays, reliably
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-037`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Make output work on every device users actually own: any sample format, any channel count, hot-plug and default-device changes.

## Objectives
1. **TASK-132** (P1): Sample-Format & Channel-Count Negotiation (f32/i16/u16/i32, N channels)
2. **TASK-133** (P1): Device Hot-Plug, Default-Device Change & Stream Error Recovery
3. **TASK-134** (P2): Honest AudioSinkPort: Remove Dead push API, Real configure, Correct Latency

## Tasks
- [ ] [TASK-132: Sample-Format & Channel-Count Negotiation (f32/i16/u16/i32, N channels)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-132-sample-format-and-channel-negotiation.md)
- [ ] [TASK-133: Device Hot-Plug, Default-Device Change & Stream Error Recovery](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-133-device-hotplug-and-stream-error-recovery.md)
- [ ] [TASK-134: Honest AudioSinkPort: Remove Dead push API, Real configure, Correct Latency](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-134-honest-audio-sink-port.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
