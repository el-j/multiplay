---
id: PLAN-004
title: Embedded Storage (redb), CQRS Event Bus & Library Indexer
status: planned
role: storage_and_state
tasks:
  - TASK-010
  - TASK-011
  - TASK-012
---

# PLAN-004: Embedded Storage (redb), CQRS Event Bus & Library Indexer

## Objective
Implement pure-Rust embedded ACID persistence using `redb`, the unidirectional Command-Query Responsibility Segregation (CQRS) event bus orchestrator, and an asynchronous local filesystem audio crawler with Lofty tag extraction.

## Architecture Context
- Vision Docs: `docs/vision/persistence_indexing_state_management.md`.
- Target Crates:
  - `crates/amp-adapter-storage`: `RedbStorageAdapter` implementing `LibraryStoragePort` using `redb` tables (`tracks_v1`, `search_index_v1`, `playlists_v1`).
  - `crates/amp-core-engine/src/events.rs` & `dispatcher.rs`: `PlayerCommand` (MPSC unbounded channel) and `PlayerEvent` (broadcast channel with N-subscribers).
  - `crates/amp-adapter-storage/src/scanner.rs`: Recursive directory walker extracting ID3/FLAC/Vorbis metadata via `lofty` into `redb`.

## Tasks
1. [TASK-010](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-010-redb-storage-adapter.md): Pure-Rust Embedded ACID Key-Value Storage Adapter (`redb`)
2. [TASK-011](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-011-cqrs-event-bus-dispatcher.md): CQRS Command-Event Bus & State Machine Orchestrator
3. [TASK-012](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-012-library-scanner-lofty-tagger.md): Local Library Filesystem Scanner with Pure-Rust Lofty Tagger

## Definition of Done
- `cargo test -p amp-adapter-storage` completes CRUD and prefix-search benchmarks under 5ms.
- CQRS bus enables non-blocking event distribution across UI, playback session, and background tasks.
- Folder crawler parses test audio directories, extracts tags, and indexes tracks into `redb`.
