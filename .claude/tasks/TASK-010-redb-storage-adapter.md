---
id: TASK-010
title: Pure-Rust Embedded ACID Key-Value Storage Adapter
planId: PLAN-004
status: pending
role: storage_and_state
verificationCommand: cargo test -p amp-adapter-storage
---

# TASK-010: Pure-Rust Embedded ACID Key-Value Storage Adapter

## Description
Implement `RedbStorageAdapter` in `crates/amp-adapter-storage`:
- Uses pure-Rust embedded ACID database `redb` (zero C compiler dependencies; replaces SQLite/RocksDB).
- Defines tables:
  - `TRACKS_TABLE`: UUID byte key to Bincode-serialized `Track`.
  - `INDEX_TABLE`: text prefix to serialized vector of track IDs.
  - `PLAYLISTS_TABLE`: playlist ID to serialized playlist.
- Implements `LibraryStoragePort`: `store_track`, `fetch_all`, `remove_track`, `search_cached`.
- Offloads disk transactions cleanly using `tokio::task::spawn_blocking`.

## Verification
```bash
cargo test -p amp-adapter-storage
```
