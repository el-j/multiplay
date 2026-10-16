---
id: TASK-066
title: Pure-Rust YouTube Stream Resolver Live Buffering Hardening
planId: PLAN-021
status: pending
role: integrations
verificationCommand: cargo test -p amp-adapter-youtube
---

# TASK-066: Pure-Rust YouTube Stream Resolver Live Buffering Hardening

## Description
Harden the YouTube stream resolution and live buffering pipeline:
1. Audio Format Selection: Ensure optimal audio-only streams (Opus/AAC bitrates 128k-256k) are selected without video demuxing overhead.
2. Progressive Range Request Buffering: Implement adaptive pre-caching using pure-Rust HTTP chunked range requests via `reqwest-rustls` into `SharedPrecacheBuffer`.
3. Symphonia Demuxer Handshake: Connect progressive byte streams directly into Symphonia's `MediaSource` abstraction to allow real-time decoding while downloading.
4. Error Resilience: Handle YouTube rate limiting, signature deciphering errors, and connection resets with clear typed error propagation.

## Verification
```bash
cargo test -p amp-adapter-youtube
```

## Acceptance Criteria
- Seamless progressive chunk buffering into the Symphonia decoder.
- No synthetic audio fallbacks on resolution failure.
- Unit and integration tests verify stream selection, error propagation, and chunk parsing.
