---
id: PLAN-014
title: Unified Configuration Management, Secure Credential Vault & Settings UI
status: planned
role: storage_and_state
tasks:
  - TASK-043
  - TASK-044
  - TASK-045
  - TASK-046
---

# PLAN-014: Unified Configuration Management, Secure Credential Vault & Settings UI

## Objective
Implement comprehensive persistent configuration management, secure zero-leak credential storage, and an interactive preferences interface as specified in `docs/vision/unified_settings_configuration_secrets_manager.md`:
1. Hierarchical TOML configuration model (`AppConfig`) with path auto-resolution and plugin extension maps.
2. Secure credential vault (`CredentialVaultPort`) with zeroize-on-drop protection for streaming authentication secrets (Spotify, Soulseek, P2P keys).
3. Atomic disk persistence and hot-reload synchronization across the CQRS EventBus.
4. Interactive multi-category Preferences modal view for terminal TUI and desktop GUI.

## Architecture Context
- Crate Targets:
  - `crates/amp-domain/src/config.rs`: `AppConfig`, `LibraryConfig`, `AudioHardwareConfig`, `StreamingProvidersConfig`, `P2PNetworkConfig`, `UiAndThemeConfig`.
  - `crates/amp-ports/src/driven.rs`: `CredentialVaultPort` driven port contract.
  - `crates/amp-adapter-storage/src/keyring.rs`: `KeyringStorageAdapter`, `SecretString` (`Zeroize`, `ZeroizeOnDrop`).
  - `crates/amp-core-engine/src/config_store.rs`: `ConfigStore`, atomic write semantics, path resolution.
  - `crates/amp-adapter-tui/src/settings_view.rs` & `crates/amp-adapter-gui`: Categorized preferences dialog with live parameter updates.
- Hexagonal Boundary: Domain and core engine never import concrete keyring adapters; all interactions flow via `CredentialVaultPort`.

## Tasks
1. [TASK-043](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-043-hierarchical-toml-configuration-domain-schema.md): Hierarchical TOML Configuration Domain Schema (`crates/amp-domain/src/config.rs`)
2. [TASK-044](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-044-credential-vault-port-and-secure-keyring-adapter.md): Driven Credential Vault Port & Secure Keyring Adapter (`crates/amp-ports` & `amp-adapter-storage`)
3. [TASK-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-045-atomic-config-storage-and-hot-reload-engine.md): Atomic Config Storage Engine & Hot-Reload Dispatcher (`crates/amp-core-engine/src/config_store.rs`)
4. [TASK-046](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-046-interactive-preferences-settings-modal.md): Interactive Preferences Settings Modal Interface for TUI and GUI

## Definition of Done
- Configuration round-trips to valid TOML with graceful fallback to defaults on corrupt files.
- Sensitive authentication credentials are encrypted/zeroized on drop and never stored in plaintext `amp.toml`.
- Config modifications persist atomically to disk with zero partial write corruption.
- Preferences modal renders and operates cleanly across both TUI and native GUI.
- 100% pure Rust verified with zero C dependencies.
