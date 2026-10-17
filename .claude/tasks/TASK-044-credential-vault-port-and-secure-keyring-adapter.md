---
id: TASK-044
title: Driven Credential Vault Port & Secure Keyring Adapter
planId: PLAN-014
status: re-opened
role: storage_and_state
auditFindings: ["ORC-01", "DAT-06"]
mappedTo: ["PLAN-041", "TASK-151"]
verificationCommand: cargo test -p amp-adapter-storage test_keyring
---

# TASK-044: Driven Credential Vault Port & Secure Keyring Adapter

## Description
Implement the secure secrets storage layer preventing cleartext credential exposure in configuration files:
1. Port Definition (`crates/amp-ports/src/driven.rs`):
   - `CredentialVaultPort`: Interface declaring `store_secret`, `get_secret`, and `delete_secret`.
2. Secure String Primitive (`crates/amp-adapter-storage/src/keyring.rs`):
   - `SecretString`: Secure container with `Zeroize` and `ZeroizeOnDrop` guarantees, erasing key material from RAM when dropped.
3. Storage Adapter (`KeyringStorageAdapter`):
   - Implements `CredentialVaultPort` using system secure keyrings (macOS Keychain, Windows Credential Manager, Linux Secret Service) with fallback to encrypted ChaCha20 master store in headless CI environments.

## Verification
```bash
cargo test -p amp-adapter-storage test_keyring
```
