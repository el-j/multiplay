---
id: TASK-012
title: Local Library Filesystem Scanner with Pure-Rust Lofty Tagger
planId: PLAN-004
status: completed
role: storage_and_state
verificationCommand: cargo test -p amp-adapter-storage test_scanner
---

# TASK-012: Local Library Filesystem Scanner with Pure-Rust Lofty Tagger

## Description
Implement the recursive directory crawler and tag extractor in `crates/amp-adapter-storage/src/scanner.rs`:
- Recursively scans configured folders for supported audio extensions (`.mp3`, `.flac`, `.wav`, `.ogg`, `.m4a`, `.aac`).
- Uses pure-Rust `lofty` to extract title, artist, album, track number, sample rate, bit depth, bitrate, duration, and embedded cover art.
- Inserts extracted tracks into `redb` storage via `LibraryStoragePort`.
- Uses filesystem events to watch for newly added, modified, or removed files.

## Verification
```bash
cargo test -p amp-adapter-storage test_scanner
```
