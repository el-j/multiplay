---
id: PLAN-012
title: Distributed Mesh Jam Session & Authenticated Mobile Remote Control
status: completed
role: integrations
tasks:
  - TASK-036
  - TASK-037
  - TASK-038
---

# PLAN-012: Distributed Mesh Jam Session & Authenticated Mobile Remote Control

## Objective
Implement decentralized collaborative playback and remote control as specified in `docs/vision/distributed_mesh_jam_mobile_control.md`:
1. Shared P2P "Jam Session" playlist with conflict-free deterministic convergence using Conflict-Free Replicated Data Types (CRDTs / RGA).
2. Pure-Rust embedded WebSocket/TCP server (`amp-adapter-remote`) accepting authenticated JSON-RPC commands and broadcasting real-time playback telemetry.
3. QR-code pairing flow rendering high-contrast ASCII QR codes into the terminal/GUI for instantaneous zero-configuration phone pairing.

## Architecture Context
- Crate Targets:
  - `crates/amp-domain/src/jam_crdt.rs`: `CollaborativeQueue`, `JamItem`, `JamOperation`, `LamportTimestamp`.
  - `crates/amp-adapter-remote`: New workspace driving adapter crate embedding pure-Rust `tokio-tungstenite` server.
  - `crates/amp-adapter-tui` & `crates/amp-adapter-gui`: Pairing modal with QR-code rendering using `qrcodegen` (100% pure Rust).
- Hexagonal Boundary: `amp-adapter-remote` translates remote WebSocket actions to `PlayerCommand`s dispatched over `EventBus`.

## Tasks
1. [TASK-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-036-collaborative-crdt-queue-state-machine.md): Collaborative CRDT Queue & Lamport State Machine
2. [TASK-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-037-embedded-websocket-remote-daemon.md): Pure-Rust Embedded WebSocket Remote Daemon (`amp-adapter-remote`)
3. [TASK-038](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-038-terminal-and-gui-qr-code-pairing.md): Terminal & GUI QR Code Pairing System with Ephemeral Tokens

## Definition of Done
- CRDT concurrent insertions, upvotes, and deletions converge identically across simulated multi-peer network topologies.
- Remote WebSocket server accepts authenticated commands (Play, Pause, Seek, Volume) and reflects state in audio playback.
- QR code renders legibly in TUI and GUI displays.
- Zero C dependencies (`./scripts/verify_pure_rust.sh` passing).
