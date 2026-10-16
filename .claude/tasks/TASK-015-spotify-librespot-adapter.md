---
id: TASK-015
title: Spotify Pure-Rust Stream Adapter
planId: PLAN-005
status: completed
role: integrations
verificationCommand: cargo test -p amp-adapter-spotify
---

# TASK-015: Spotify Pure-Rust Stream Adapter

## Description
Implement `PureSpotifyAdapter` in `crates/amp-adapter-spotify`:
- Uses pure-Rust `librespot-core` and `librespot-playback` with native crypto.
- Implements authentication via stored credentials or OAuth tokens.
- Implements `StreamResolverPort` to resolve Spotify track URIs to decrypted Vorbis/Ogg byte streams.
- Exposes provider status and handles rate limiting / authentication renewal.

## Verification
```bash
cargo test -p amp-adapter-spotify
```
