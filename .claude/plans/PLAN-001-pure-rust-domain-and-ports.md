---
id: PLAN-001
title: Pure-Rust Toolchain, Domain Models & Hexagonal Port Contracts
status: completed
role: architecture
tasks:
  - TASK-001
  - TASK-002
  - TASK-003
---

# PLAN-001: Pure-Rust Toolchain, Domain Models & Hexagonal Port Contracts

## Objective
Establish the pure-Rust Cargo workspace, enforce toolchain constraints to prevent any native C/C++ compilation, and implement the complete zero-dependency domain models and hexagonal driving/driven port contracts.

## Architecture Context
- Vision Docs: `docs/vision/system_architecture_topology.md`, `docs/vision/domain_models_port_contracts.md`, `docs/vision/multi_source_search_filters_source_badges.md`.
- Target Crates:
  - Root workspace `Cargo.toml` and `.cargo/config.toml`
  - `crates/amp-domain`: Entities, Value Objects, SourceKind, SourceFilterMask, TrackMetadata, BufferConfiguration.
  - `crates/amp-ports`: Driving ports (`PlayerControlPort`, `PlaylistManagementPort`, `UniversalSearchPort`, `FilteredSearchPort`, `TrackDownloadPort`) and Driven ports (`AudioSinkPort`, `StreamResolverPort`, `ProviderSearchPort`, `LibraryStoragePort`, `AudioSourceStream`, `AmpAudioPlugin`, `DspFilterHook`).

## Tasks
1. [TASK-001](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-001-cargo-workspace-and-toolchain.md): Root Cargo Workspace & Pure-Rust Toolchain Constraints
2. [TASK-002](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-002-domain-models-and-entities.md): Pure Domain Models, Entities & Source Badges (`amp-domain`)
3. [TASK-003](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-003-hexagonal-port-contracts.md): Driving & Driven Hexagonal Port Traits (`amp-ports`)

## Definition of Done
- `cargo check --workspace` compiles cleanly with zero warnings.
- `./scripts/verify_pure_rust.sh` exits 0 with zero forbidden C-wrapper crates.
- `#![forbid(unsafe_code)]` declared in `crates/amp-domain` and `crates/amp-ports`.
