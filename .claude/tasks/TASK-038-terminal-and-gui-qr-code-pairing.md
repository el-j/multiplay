---
id: TASK-038
title: Terminal & GUI QR Code Pairing System with Ephemeral Tokens
planId: PLAN-012
status: completed
role: frontend_ui
verificationCommand: cargo test -p amp-adapter-remote test_pairing
---

# TASK-038: Terminal & GUI QR Code Pairing System with Ephemeral Tokens

## Description
Implement the instant zero-configuration mobile pairing engine:
1. `PairingSession`: Generates ephemeral authentication tokens and encodes local network WebSocket endpoint URLs (`ws://<local_ip>:<port>?token=<ephemeral_token>`).
2. Pure-Rust QR Generator: Uses the 100% pure-Rust `qrcodegen` crate to compute two-dimensional QR matrices without external libraries.
3. Rendering Adapters:
   - TUI: Dense UTF-8 half-block matrix renderer (`▄`, `█`, `▀`) inside a Ratatui dialog modal.
   - GUI: Pure-Rust GPU bitmap painter widget for high-contrast mobile camera scanning.

## Verification
```bash
cargo test -p amp-adapter-remote test_pairing
```
