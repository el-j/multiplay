---
id: TASK-045
title: Atomic Config Storage Engine & Hot-Reload Dispatcher
planId: PLAN-014
status: planned
role: storage_and_state
verificationCommand: cargo test -p amp-core-engine test_config_store
---

# TASK-045: Atomic Config Storage Engine & Hot-Reload Dispatcher

## Description
Implement atomic configuration file storage and real-time synchronization in `crates/amp-core-engine/src/config_store.rs`:
1. `ConfigStore`:
   - Resolves canonical configuration directory (`~/.config/amp/amp.toml` on macOS/Linux, `%APPDATA%\amp\amp.toml` on Windows).
   - Initializes default configuration if file does not exist.
   - Robust error recovery: Falls back to defaults with diagnostics if disk file contains syntax errors.
2. Atomic File Replacement:
   - Writes mutations to `.tmp` file and replaces target atomically via filesystem rename, preventing file corruption on unexpected power loss.
3. Fast Live Reader Handle:
   - Exposes `Arc<RwLock<AppConfig>>` for zero-overhead in-memory reads on high-frequency threads.
   - Emits `PlayerEvent::ConfigChanged` over the CQRS `EventBus` when settings are mutated.

## Verification
```bash
cargo test -p amp-core-engine test_config_store
```
