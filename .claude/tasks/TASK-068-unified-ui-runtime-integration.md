---
id: TASK-068
title: Unified UI Runtime Integration Across CLI, TUI, and GUI
planId: PLAN-022
status: pending
role: architecture
verificationCommand: cargo test -p amp-app -p amp-adapter-tui -p amp-adapter-gui
---

# TASK-068: Unified UI Runtime Integration Across CLI, TUI, and GUI

## Description
Unify the driving presentation layers around the single authoritative domain playback engine:
1. Shared Engine Architecture: Ensure both the Ratatui TUI (`amp-adapter-tui`), the desktop egui application (`amp-adapter-gui`), and the CLI runner (`amp-app`) interact with the same underlying `PlaybackSession` and CQRS command bus.
2. Event Stream Propagation: Stream playback state events (TrackChanged, PositionUpdated, VolumeChanged, EqualizerChanged, ErrorOccurred) from the audio engine to UI subscribers via crossbeam/flume channels.
3. Interactive Controls Synchronization: Ensure timeline scrubbing, playlist reordering, volume adjustments, and EQ slider changes synchronize seamlessly with the audio engine in real time.
4. Clean Shutdown: Ensure terminal state restoration (raw mode reset, cursor show) and audio device teardown occur cleanly on SIGINT, SIGTERM, and window close.

## Verification
```bash
cargo test -p amp-app -p amp-adapter-tui -p amp-adapter-gui
```

## Acceptance Criteria
- Both TUI and GUI interfaces consume the exact same core engine domain events.
- UI state updates smoothly reflect real hardware playback position.
- Tests verify signal handling, event channel dispatch, and graceful shutdown.
