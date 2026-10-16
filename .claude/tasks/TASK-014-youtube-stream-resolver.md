---
id: TASK-014
title: YouTube Pure-Rust Stream Resolver & Search Adapter
planId: PLAN-005
status: completed
role: integrations
verificationCommand: cargo test -p amp-adapter-youtube
---

# TASK-014: YouTube Pure-Rust Stream Resolver & Search Adapter

## Description
Implement `PureYouTubeAdapter` in `crates/amp-adapter-youtube`:
- Implements `ProviderSearchPort` and `StreamResolverPort` using pure-Rust `rusty-ytdl` with `rustls-tls` (no openssl-sys).
- `query(&str)` searches YouTube for audio streams, mapping metadata, video IDs, titles, artists/channels, durations, and thumbnails.
- `resolve_stream(&AudioSource)` selects highest audio quality format and returns `Box<dyn AudioSourceStream>`.
- Error handling for age restrictions, regional blocks, and network failures.

## Verification
```bash
cargo test -p amp-adapter-youtube
```
