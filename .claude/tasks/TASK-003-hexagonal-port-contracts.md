---
id: TASK-003
title: Driving & Driven Hexagonal Port Traits
planId: PLAN-001
status: pending
role: architecture
verificationCommand: cargo test -p amp-ports
---

# TASK-003: Driving & Driven Hexagonal Port Traits

## Description
Implement the decoupled port contracts in `crates/amp-ports`:
- Inbound / Driving Ports:
  - `PlayerControlPort`: play_track, toggle_pause, stop, seek_relative, seek_absolute, current_state, current_position.
  - `PlaylistManagementPort`: add_to_playlist, remove_from_playlist, clear_playlist, move_track, get_playlist.
  - `UniversalSearchPort`: execute_search.
  - `FilteredSearchPort`: search with `SearchQueryOptions` (filter mask and timeout).
  - `TrackDownloadPort`: queue_download, cancel_download.
- Outbound / Driven Ports:
  - `AudioSourceStream`: `Read + Seek + Send + Sync`.
  - `StreamResolverPort`: handles_source, resolve_stream.
  - `ProviderSearchPort`: provider_id, query.
  - `AudioSinkPort`: configure, start, pause, stop, push_interleaved_samples, get_buffer_latency.
  - `LibraryStoragePort`: store_track, fetch_all, remove_track, search_cached.
  - `AmpAudioPlugin`, `DspFilterHook`.

## Constraints
- Depends only on `amp-domain`, `async-trait`, `thiserror`.
- `#![forbid(unsafe_code)]`.

## Verification
```bash
cargo test -p amp-ports
```
