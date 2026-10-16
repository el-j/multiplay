---
id: PLAN-008
title: P2P Mesh, Soulseek Protocol & Runtime Composition Root
status: planned
role: system_composition
tasks:
  - TASK-023
  - TASK-024
  - TASK-025
---

# PLAN-008: P2P Mesh, Soulseek Protocol & Runtime Composition Root

## Objective
Implement pure-Rust network adapters for Soulseek binary protocol and encrypted P2P private mesh (`libp2p` with Noise and Yamux), assemble the main runtime composition root (`amp-app`), and establish end-to-end integration tests.

## Architecture Context
- Vision Docs: `docs/vision/runtime_composition_build_verification.md`, `docs/vision/system_architecture_topology.md`.
- Target Crates:
  - `crates/amp-adapter-soulseek`: Soulseek binary protocol adapter for searching and streaming.
  - `crates/amp-adapter-p2p`: Encrypted P2P private mesh node using `libp2p`.
  - `crates/amp-app/src/main.rs`: Composition root initializing ports, adapters, CQRS bus, CPAL audio sink, and launching the TUI event loop.

## Tasks
1. [TASK-023](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-023-soulseek-protocol-adapter.md): Pure-Rust Soulseek Protocol Search & Transfer Adapter
2. [TASK-024](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-024-encrypted-p2p-mesh-adapter.md): Encrypted Private P2P Mesh Adapter (`libp2p`)
3. [TASK-025](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-025-runtime-composition-root.md): Main Runtime Composition Root (`amp-app`) & System Integration

## Definition of Done
- `cargo run -p amp-app` boots the full audio player, starts CPAL audio sink, connects event bus, and renders the Winamp TUI.
- Integration test suite passes with zero warnings.
- Verification script `./scripts/verify_pure_rust.sh` confirms 100% zero-C toolchain compliance.
