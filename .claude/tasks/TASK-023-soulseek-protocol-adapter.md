---
id: TASK-023
title: Pure-Rust Soulseek Protocol Search & Transfer Adapter
planId: PLAN-008
status: re-opened
role: system_composition
auditFindings: ["ORC-01", "SRC-01", "SRC-09"]
mappedTo: ["PLAN-045", "TASK-162", "TASK-163", "TASK-164", "TASK-165", "TASK-166"]
verificationCommand: cargo test -p amp-adapter-soulseek
---

# TASK-023: Pure-Rust Soulseek Protocol Search & Transfer Adapter

## Description
Implement `SoulseekAdapter` in `crates/amp-adapter-soulseek`:
- Pure-Rust implementation of the binary Soulseek P2P network protocol.
- Connects to central server, manages login handshake, heartbeats, and user status.
- Implements `ProviderSearchPort` to query peer shared folders and decode result packets (bitrate, file size, speed, slots).
- Implements `StreamResolverPort` to initiate direct peer-to-peer file transfer connections and stream audio bytes directly into `SharedPrecacheBuffer`.

## Verification
```bash
cargo test -p amp-adapter-soulseek
```
