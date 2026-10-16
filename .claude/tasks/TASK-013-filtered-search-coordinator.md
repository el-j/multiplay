---
id: TASK-013
title: Parallel Multi-Source Search Coordinator with Filter Mask
planId: PLAN-005
status: completed
role: integrations
verificationCommand: cargo test -p amp-core-engine test_search_coordinator
---

# TASK-013: Parallel Multi-Source Search Coordinator with Filter Mask

## Description
Implement `FilteredSearchCoordinator` in `crates/amp-core-engine/src/filtered_search.rs`:
- Implements `FilteredSearchPort` trait.
- Registers multiple providers with their corresponding `SourceKind`.
- Uses `tokio::task::JoinSet` to concurrently query only active providers according to `SourceFilterMask`.
- Enforces an aggregated search timeout (e.g., 3.5 seconds) using `tokio::select!` and aborts running workers on expiry.
- Merges results into a unified `Vec<Track>` collection.

## Verification
```bash
cargo test -p amp-core-engine test_search_coordinator
```
