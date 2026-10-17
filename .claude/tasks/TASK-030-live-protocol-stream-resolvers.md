---
id: TASK-030
title: Live Protocol Stream Handlers for Spotify, Soulseek TCP, and Encrypted P2P
planId: PLAN-010
status: re-opened
role: integrations
auditFindings: ["ORC-01", "SRC-01", "SRC-02"]
mappedTo: ["PLAN-039", "PLAN-043", "PLAN-045"]
verificationCommand: cargo test -p amp-adapter-spotify -p amp-adapter-soulseek -p amp-adapter-p2p
---

# TASK-030: Live Protocol Stream Handlers for Spotify, Soulseek TCP, and Encrypted P2P

## Description
Replace mock / simulated empty stream cursors in secondary stream resolvers:
1. `crates/amp-adapter-spotify`:
   - Connect authenticated session to live audio key decryption stream.
2. `crates/amp-adapter-soulseek`:
   - Establish direct peer TCP socket transfer using binary message codes 40 (`TransferRequest`) and 41 (`TransferResponse`) to stream raw audio bytes.
3. `crates/amp-adapter-p2p`:
   - Establish encrypted peer data transfer stream for shared lossless hashes.

## Verification
```bash
cargo test -p amp-adapter-spotify -p amp-adapter-soulseek -p amp-adapter-p2p
```
