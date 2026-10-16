---
id: TASK-025
title: Main Runtime Composition Root & System Integration
planId: PLAN-008
status: pending
role: system_composition
verificationCommand: ./scripts/verify_pure_rust.sh && cargo test --workspace
---

# TASK-025: Main Runtime Composition Root & System Integration

## Description
Implement the runtime composition root in `crates/amp-app/src/main.rs`:
- Initializes CQRS `EventBus`.
- Initializes `RedbStorageAdapter` with local data directory.
- Initializes `CpalAudioSink` and launches real-time hardware playback thread.
- Configures `FilteredSearchCoordinator` and registers YouTube, Spotify, Soulseek, and P2P adapters.
- Spawns background command dispatcher task.
- Boots the Ratatui modular TUI terminal loop with keyboard/mouse event listener.
- Handles clean shutdown on SIGINT / `Ctrl + C`.
- Performs full end-to-end integration testing.

## Verification
```bash
./scripts/verify_pure_rust.sh && cargo test --workspace
```
