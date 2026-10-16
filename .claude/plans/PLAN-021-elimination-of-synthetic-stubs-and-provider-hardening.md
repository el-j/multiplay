---
id: PLAN-021
title: Elimination of Synthetic Stubs & Production Provider Hardening
status: completed
role: integrations
tasks:
  - TASK-063
  - TASK-064
  - TASK-065
  - TASK-066
---

# PLAN-021: Elimination of Synthetic Stubs & Production Provider Hardening

## Objective
Completely eliminate all synthetic audio generators, hard-coded search query branches, and mock payloads across all provider adapters. Turn every integration into a production-grade pure-Rust implementation with explicit, typed error handling and fail-fast guarantees.

## Architecture Context
- **Audit Grounding (`docs/orchestration-audit.md`)**:
  - Section 3.1 & 6: Spotify adapter generated synthetic WAV fallbacks.
  - Section 3.2 & 6: P2P mesh adapter generated "P2P Mesh Anthem" and synthetic payloads.
  - Section 3.3 & 6: Soulseek adapter returned hard-coded "aphex" query results and synthetic audio.
- **Remediation Strategy (`docs/orchestration-remediation-plan.md` Phase 1)**:
  - Remove all synthetic audio generation code paths.
  - Implement true protocol framing, authentic cryptographic handshakes, and strict error propagation.
  - Retain 100% pure Rust compliance (`#![forbid(unsafe_code)]`).

## Tasks
1. [TASK-063](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-063-spotify-adapter-stub-elimination-and-session-hardening.md): Pure-Rust Spotify Adapter Stub Elimination & Session Hardening
2. [TASK-064](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-064-soulseek-protocol-real-search-and-frame-state-machine.md): Pure-Rust Soulseek Protocol Search & Message Frame State Machine
3. [TASK-065](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-065-p2p-mesh-cryptographic-handshake-and-block-transfer.md): Pure-Rust P2P Mesh Cryptographic Handshake & Block Transfer
4. [TASK-066](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-066-youtube-stream-resolver-live-buffering-hardening.md): Pure-Rust YouTube Stream Resolver Live Buffering Hardening

## Definition of Done
- Zero synthetic audio generators or placeholder strings remain in any provider adapter.
- All network or authentication failures return typed errors to the domain layer.
- Tests verify real protocol encoding, cryptographic handshakes, and error handling.
