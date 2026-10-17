# PLAN-037: Unified Core Playback Engine (PlayerEngine) & Transport Correctness

## Plan Overview
- **ID:** `PLAN-037`
- **Title:** Unified Core Playback Engine (PlayerEngine) & Transport Correctness
- **Role:** `audio_engine, architecture`
- **Milestone:** M1 — It plays, reliably
- **Priority:** P0
- **Status:** Pending
- **Dependencies:** `PLAN-036`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Move playback ownership into `amp-core-engine`: one engine implementing the driving ports, consumed by GUI, TUI and remote alike, with correct pause/stop/seek, output-clock position, async resolution and surfaced errors.

## Objectives
1. **TASK-125** (P0): PlayerEngine: State Machine Implementing PlayerControlPort & PlaylistManagementPort
2. **TASK-126** (P1): Replace Both UI Audio Drivers with PlayerEngine; Delete Duplicates
3. **TASK-127** (P1): Transport Correctness: Epoch-Flushed Ring, Real Pause, No Lost Chunks
4. **TASK-128** (P1): Output-Clock Playback Position & Output-Aligned Visualizer Tap
5. **TASK-129** (P0): Async Stream Resolution & Preloading Off the Audio Worker
6. **TASK-130** (P2): Typed Port Errors & User-Visible Error Events
7. **TASK-131** (P1): Queue Semantics: Next/Previous/Repeat/Shuffle Owned by the Engine

## Tasks
- [ ] [TASK-125: PlayerEngine: State Machine Implementing PlayerControlPort & PlaylistManagementPort](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-125-player-engine-implementing-driving-ports.md)
- [ ] [TASK-126: Replace Both UI Audio Drivers with PlayerEngine; Delete Duplicates](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-126-replace-duplicated-ui-audio-drivers-with-engine.md)
- [ ] [TASK-127: Transport Correctness: Epoch-Flushed Ring, Real Pause, No Lost Chunks](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-127-transport-flush-pause-stop-seek.md)
- [ ] [TASK-128: Output-Clock Playback Position & Output-Aligned Visualizer Tap](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-128-output-clock-position-and-aligned-meters.md)
- [ ] [TASK-129: Async Stream Resolution & Preloading Off the Audio Worker](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-129-async-resolution-and-preloading-off-audio-thread.md)
- [ ] [TASK-130: Typed Port Errors & User-Visible Error Events](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-130-typed-errors-and-error-events.md)
- [ ] [TASK-131: Queue Semantics: Next/Previous/Repeat/Shuffle Owned by the Engine](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-131-queue-semantics-next-prev-repeat-shuffle.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
