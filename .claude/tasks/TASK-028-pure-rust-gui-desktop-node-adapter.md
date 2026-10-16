---
id: TASK-028
title: Pure-Rust GUI Desktop Node Adapter Architecture
planId: PLAN-009
status: in_progress
role: frontend_gui
verificationCommand: ./scripts/verify_pure_rust.sh && cargo check --workspace
---

# TASK-028: Pure-Rust GUI Desktop Node Adapter Architecture

## Description
Architect the dedicated GUI desktop node driving adapter (`crates/amp-adapter-gui`) as outlined in `docs/vision/system_architecture_topology.md`:
1. Maintain strict pure-Rust `#![forbid(unsafe_code)]` compliance with zero C dynamic/static linkage.
2. Bind to driving ports `PlayerControlPort`, `UniversalSearchPort`, and `PlaylistManagementPort`.
3. Support classic Winamp bitmap skin rendering (`main.bmp`, `cbuttons.bmp`, `eqmain.bmp`, `pledit.bmp`) using in-memory ZIP decompression from `SkinLoader`.
4. Render 60 FPS hardware/software blitted spectrum visualizer and magnetic window dragging.

## Verification
```bash
./scripts/verify_pure_rust.sh && cargo check --workspace
```
