---
id: PLAN-022
title: End-to-End Audio Pipeline Integration & Cross-Platform Runtime Wiring
status: in_progress
role: audio_engine
tasks:
  - TASK-067
  - TASK-068
  - TASK-069
---

# PLAN-022: End-to-End Audio Pipeline Integration & Cross-Platform Runtime Wiring

## Objective
Wire all individual components (resolvers, demuxers, decoders, DSP chain, ring buffers, and CPAL hardware sinks) into an unbroken, real-time verified audio pipeline powering both CLI, TUI, and GUI frontends.

## Architecture Context
- **Audit Grounding (`docs/orchestration-audit.md`)**:
  - Section 3.4 & 8: Runtime composition root was not fully connected end-to-end; app shell used sample tracks without a proven live decoding and hardware sink pipeline.
- **Remediation Strategy (`docs/orchestration-remediation-plan.md` Phase 2)**:
  - Wire the decode worker to the audio sink using a verified lock-free ring-buffer contract.
  - Connect TUI and GUI presentation layers to a single unified playback coordinator.
  - Implement gapless track advancement and underrun resilience.

## Tasks
1. [TASK-067](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-067-unified-playback-composition-root.md): Unified Playback Composition Root & Hardware Audio Wiring
2. [TASK-068](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-068-unified-ui-runtime-integration.md): Unified UI Runtime Integration Across CLI, TUI, and GUI
3. [TASK-069](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-069-gapless-playback-seek-and-underrun-resilience.md): Gapless Playback, Accurate Seeking & Underrun Resilience

## Definition of Done
- A real track can move from source selection to decoded buffer, DSP processing, and hardware output.
- Both TUI and GUI react to identical core engine events.
- Zero audio callback memory allocations or lock waits.
