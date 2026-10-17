---
id: TASK-063
title: Pure-Rust Spotify Adapter Stub Elimination & Session Hardening
planId: PLAN-021
status: re-opened
role: integrations
auditFindings: ["ORC-01", "SRC-01"]
mappedTo: ["PLAN-046", "TASK-168"]
verificationCommand: cargo test -p amp-adapter-spotify
---

# TASK-063: Pure-Rust Spotify Adapter Stub Elimination & Session Hardening

## Description
Eliminate synthetic fallback audio generation from the Spotify adapter identified in the audit (`docs/orchestration-audit.md` section 3.1 & 6):
1. Stub Removal: Remove the generated PCM WAV fallback (`generate_synthetic_stream`) from production code paths in `crates/amp-adapter-spotify/src/lib.rs`.
2. Authenticated Session Flow: Require valid credentials or active session token for track resolution; if unauthenticated or offline, return an explicit, typed error (`SpotifyError::Unauthenticated` or `SpotifyError::TrackUnavailable`) rather than silently synthesizing fake audio.
3. Audio Key & Stream Resolution: Structure stream resolution to use pure-Rust librespot protocol mechanisms for encrypted chunk retrieval and decryption keys, fail-fast on permission or network errors.
4. Comprehensive Testing: Write unit tests covering authentication failure, expired session handling, network timeout handling, and successful track metadata retrieval with zero synthetic WAV generation.

## Verification
```bash
cargo test -p amp-adapter-spotify
```

## Acceptance Criteria
- Zero synthetic sine-wave/WAV generation code remains in production stream resolution.
- Explicit typed errors returned when credentials or network connectivity are absent.
- 100% pure-Rust compliance verified with zero C/C++ linkage.
