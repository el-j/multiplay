---
id: TASK-002
title: Pure Domain Models, Entities & Source Badges
planId: PLAN-001
status: pending
role: architecture
verificationCommand: cargo test -p amp-domain
---

# TASK-002: Pure Domain Models, Entities & Source Badges

## Description
Implement the zero-dependency pure domain core in `crates/amp-domain`. This crate defines the domain entities, value objects, source kinds, and source filter masks.

## Key Types
- `TrackId(pub Uuid)`
- `AudioSource`: `LocalFile`, `NetworkSmb`, `DirectHttp`, `YouTube`, `Spotify`, `Bandcamp`, `SoundCloud`.
- `TrackMetadata`: title, artist, album, track_number, duration, sample_rate, channels, bit_depth, bitrate_kbps, cover_art_uri.
- `Track`: id, source, metadata, date_added_epoch.
- `PlaybackState`: `Stopped`, `Buffering { progress_percent: u8 }`, `Playing`, `Paused`.
- `AudioFrameChunk`: interleaved normalized samples `Vec<f32>`, channels, sample_rate.
- `BufferConfiguration`: precache capacity, ring buffer samples, watermarks.
- `SourceKind`: `LocalFile`, `NetworkSmb`, `YouTube`, `Spotify`, `Bandcamp`, `SoundCloud`, `Soulseek`, `P2PShared` with `badge_icon()` and `display_label()`.
- `SourceFilterMask`: dynamic toggleable set of enabled `SourceKind`s.

## Constraints
- `#![forbid(unsafe_code)]` at crate root.
- Zero dependencies except `serde`, `uuid`, and standard library.

## Verification
```bash
cargo test -p amp-domain
```
