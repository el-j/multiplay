---
id: TASK-055
title: Pure-Rust Metadata Extraction & Playlist Item Probing
planId: PLAN-017
status: completed
role: storage_and_state
verificationCommand: cargo test -p amp-adapter-storage test_probe_metadata
---

# TASK-055: Pure-Rust Metadata Extraction & Playlist Item Probing

## Description
Extract comprehensive audio metadata for dropped media items using pure-Rust decoders and taggers:
1. Pure-Rust Lofty & Symphonia Probe: Inspect file header to extract:
   - Track Title, Artist, Album, Year, Genre, Track Number.
   - Technical specifications: Duration (seconds), Bitrate (kbps), Channels (Stereo/Mono), Sample Rate (Hz).
2. Clean Fallback Heuristics: If tags are absent or unreadable:
   - Parse file stem according to common patterns: `"Artist - Title"`, `"01 - Title"`, `"Artist - Album - 01 - Title"`.
   - Set sensible fallback values (e.g. Unknown Artist, Title derived from filename).
3. Asynchronous Non-Blocking Probing: Perform metadata extraction on background worker thread so the UI remains 60 FPS responsive even when hundreds of tracks are dropped simultaneously.

## Verification
```bash
cargo test -p amp-adapter-storage test_probe_metadata
```
