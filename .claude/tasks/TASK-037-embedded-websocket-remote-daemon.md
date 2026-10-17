---
id: TASK-037
title: Pure-Rust Embedded WebSocket Remote Daemon
planId: PLAN-012
status: re-opened
role: integrations
auditFindings: ["ORC-01", "SEC-01", "SEC-02"]
mappedTo: ["PLAN-035", "TASK-116", "TASK-117", "TASK-118", "TASK-119"]
verificationCommand: cargo test -p amp-adapter-remote
---

# TASK-037: Pure-Rust Embedded WebSocket Remote Daemon

## Description
Build the driving adapter crate `crates/amp-adapter-remote` providing a lightweight, pure-Rust WebSocket control server:
1. Pure-Rust stack: Uses `tokio-tungstenite`, `futures-util`, and `serde_json` with zero C dynamic/static dependencies.
2. Protocol Schema:
   - Inbound `RemoteAction`: `GetPlaybackStatus`, `Play`, `Pause`, `Next`, `Previous`, `SetVolume`, `Seek`, `EnqueueTrack`, `VoteSkip`.
   - Outbound `RemoteEventBroadcast`: Real-time state updates including track metadata, elapsed duration, volume, and active peer count.
3. EventBus Binding: Decoupled translation of WebSocket actions into CQRS `PlayerCommand`s dispatched over `EventBus`.

## Verification
```bash
cargo test -p amp-adapter-remote
```
