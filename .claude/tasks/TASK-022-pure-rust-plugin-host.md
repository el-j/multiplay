---
id: TASK-022
title: Safe Pure-Rust Plugin Host & Capability Registry
planId: PLAN-007
status: pending
role: extensibility
verificationCommand: cargo test -p amp-core-engine test_plugin_registry
---

# TASK-022: Safe Pure-Rust Plugin Host & Capability Registry

## Description
Implement `PluginRegistry` in `crates/amp-core-engine/src/plugin_registry.rs`:
- Eliminates unsafe native C shared-library loading (`libloading` / `.so` / `.dylib`).
- Supports static compile-time pure-Rust plugin registration and WASM component sandbox integration.
- Indexes plugins by capabilities: `AudioSourceProvider`, `DspProcessor`, `VisualizerEngine`, `MetadataEnricher`.
- Allows querying plugins dynamically: `list_by_capability`.
- Thread-safe registry using `Arc<RwLock<HashMap<String, Arc<dyn AmpAudioPlugin>>>>`.

## Verification
```bash
cargo test -p amp-core-engine test_plugin_registry
```
