---
id: PLAN-005
title: Multi-Source Search Aggregator, Stream Resolvers & Downloader
status: planned
role: integrations
tasks:
  - TASK-013
  - TASK-014
  - TASK-015
  - TASK-016
---

# PLAN-005: Multi-Source Search Aggregator, Stream Resolvers & Downloader

## Objective
Implement parallel fan-out search across multiple audio sources with exclusion filters, pure-Rust stream resolvers for YouTube (`rusty-ytdl`) and Spotify (`librespot-core`), and an offline track downloader with Lofty ID3/Vorbis tag embedding.

## Architecture Context
- Vision Docs: `docs/vision/providers_search_downloader.md`, `docs/vision/multi_source_search_filters_source_badges.md`.
- Target Crates:
  - `crates/amp-core-engine/src/filtered_search.rs`: `FilteredSearchCoordinator` utilizing `tokio::task::JoinSet` with `SourceFilterMask` and timeout enforcement.
  - `crates/amp-adapter-youtube`: Pure-Rust YouTube stream resolver & search provider via `rusty-ytdl`.
  - `crates/amp-adapter-spotify`: Spotify stream adapter via `librespot-core`.
  - `crates/amp-core-engine/src/downloader.rs`: `OfflineTrackDownloader` streaming remote audio to disk and applying Lofty metadata tags.

## Tasks
1. [TASK-013](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-013-filtered-search-coordinator.md): Parallel Multi-Source Search Coordinator with Filter Mask
2. [TASK-014](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-014-youtube-stream-resolver.md): YouTube Pure-Rust Stream Resolver & Search Adapter (`rusty-ytdl`)
3. [TASK-015](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-015-spotify-librespot-adapter.md): Spotify Pure-Rust Stream Adapter (`librespot-core`)
4. [TASK-016](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-016-offline-downloader-tagger.md): Offline Stream Downloader Pipeline with Pure-Rust Lofty Tagging

## Definition of Done
- Multi-source search queries all enabled providers concurrently and terminates cleanly within timeout window.
- Stream resolvers return valid `AudioSourceStream` (`Read + Seek + Send + Sync`) handles.
- Downloader saves stream bytes to destination files and injects complete ID3v2/Vorbis tags.
