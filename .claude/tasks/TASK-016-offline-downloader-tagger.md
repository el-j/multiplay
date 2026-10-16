---
id: TASK-016
title: Offline Stream Downloader Pipeline with Pure-Rust Lofty Tagging
planId: PLAN-005
status: pending
role: integrations
verificationCommand: cargo test -p amp-core-engine test_downloader
---

# TASK-016: Offline Stream Downloader Pipeline with Pure-Rust Lofty Tagging

## Description
Implement `OfflineTrackDownloader` in `crates/amp-core-engine/src/downloader.rs`:
- Reads stream bytes chunk-by-chunk from any `StreamResolverPort` adapter.
- Asynchronously streams bytes into a destination file using `tokio::fs::File`.
- Emits download progress events via `EventBus` (`PlayerEvent::DownloadProgress`).
- On download completion, uses pure-Rust `lofty` to inject complete ID3v2/Vorbis tags (artist, title, album, year, artwork).
- Returns the local `PathBuf` for immediate local playback or library addition.

## Verification
```bash
cargo test -p amp-core-engine test_downloader
```
