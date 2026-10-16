---
id: TASK-001
title: Root Cargo Workspace & Pure-Rust Toolchain Constraints
planId: PLAN-001
status: in_progress
role: architecture
verificationCommand: ./scripts/verify_pure_rust.sh && cargo check --workspace
---

# TASK-001: Root Cargo Workspace & Pure-Rust Toolchain Constraints

## Description
Create the root `Cargo.toml` workspace manifest with member crates, shared workspace dependencies with strict pure-Rust feature flags, and `.cargo/config.toml` enforcing `OPENSSL_NO_VENDOR = "1"`.

## Requirements
- Workspace members:
  - `crates/amp-domain`
  - `crates/amp-ports`
  - `crates/amp-core-engine`
  - `crates/amp-adapter-cpal`
  - `crates/amp-adapter-decoder`
  - `crates/amp-adapter-http`
  - `crates/amp-adapter-youtube`
  - `crates/amp-adapter-spotify`
  - `crates/amp-adapter-soulseek`
  - `crates/amp-adapter-p2p`
  - `crates/amp-adapter-storage`
  - `crates/amp-adapter-tui`
  - `crates/amp-app`
- All dependencies must be configured with pure-Rust flags (`rustls-tls`, `ring`, no C linkage).
- Pure-Rust audit script `./scripts/verify_pure_rust.sh` must succeed.

## Verification
```bash
./scripts/verify_pure_rust.sh && cargo check --workspace
```
