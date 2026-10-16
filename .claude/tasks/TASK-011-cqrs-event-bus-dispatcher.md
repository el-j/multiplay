---
id: TASK-011
title: CQRS Command-Event Bus & State Machine Orchestrator
planId: PLAN-004
status: pending
role: storage_and_state
verificationCommand: cargo test -p amp-core-engine test_cqrs
---

# TASK-011: CQRS Command-Event Bus & State Machine Orchestrator

## Description
Implement the unidirectional CQRS event architecture in `crates/amp-core-engine/`:
- `events.rs`:
  - `PlayerCommand`: PlayTrack, TogglePause, Stop, SeekRelative, SeekAbsolute, SetVolume, AddToQueue, RemoveFromQueue, ClearQueue, TriggerDownload.
  - `PlayerEvent`: TrackStarted, PlaybackStateChanged, PositionUpdated, SpectrumDataReady, BufferHealth, DownloadProgress, ErrorNotification.
- `dispatcher.rs`: `EventBus` managing:
  - MPSC unbounded channel for incoming commands from UI.
  - Broadcast channel for fan-out events to N-subscribers (UI renderers, audio loop, background downloaders).

## Verification
```bash
cargo test -p amp-core-engine test_cqrs
```
