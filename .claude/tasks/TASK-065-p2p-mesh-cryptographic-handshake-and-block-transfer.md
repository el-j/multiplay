---
id: TASK-065
title: Pure-Rust P2P Mesh Cryptographic Handshake & Block Transfer
planId: PLAN-021
status: re-opened
role: integrations
auditFindings: ["ORC-01", "SRC-16"]
mappedTo: ["PLAN-046", "TASK-167"]
verificationCommand: cargo test -p amp-adapter-p2p
---

# TASK-065: Pure-Rust P2P Mesh Cryptographic Handshake & Block Transfer

## Description
Replace placeholder P2P mesh logic with real cryptographic handshake and peer-to-peer audio block transfers (`docs/orchestration-audit.md` section 3.2 & 6):
1. Remove Fabricated Content: Remove hard-coded "P2P Mesh Anthem" query response and synthetic WAV audio generation in `crates/amp-adapter-p2p/src/lib.rs`.
2. Ed25519 Peer Identity & Handshake: Implement mutual node authentication using pure-Rust Ed25519 signatures and ChaCha20-Poly1305 encrypted framing for peer-to-peer communication.
3. Content-Addressed Block Transfer: Model audio streaming as content-addressed chunk transfers verified against BLAKE3 hashes.
4. Robust Peer Failure Modes: Return explicit `P2pError::PeerUntrusted`, `P2pError::HashMismatch`, or `P2pError::ChunkUnavailable` when peer nodes disconnect or data verification fails, without any placeholder fallbacks.

## Verification
```bash
cargo test -p amp-adapter-p2p
```

## Acceptance Criteria
- Zero fabricated tracks or synthetic WAV fallbacks in `amp-adapter-p2p`.
- Cryptographically authenticated peer handshake using pure-Rust primitives.
- Comprehensive unit tests verifying valid handshakes, signature rejection on tampering, and hash-verified block transfers.
