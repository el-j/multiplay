---
id: TASK-062
title: Domain Contract Alignment & Error Standardization
planId: PLAN-020
status: completed
role: architecture
verificationCommand: cargo test -p amp-domain -p amp-ports
---

# TASK-062: Domain Contract Alignment & Error Standardization

## Description
Resolve inconsistencies between architectural port specifications and concrete domain implementations identified in the audit:
1. `SourceKind` Alignment: In `crates/amp-domain/src/source_badge.rs`, add the `DirectHttp` variant to `SourceKind` and update `SourceFilterMask` so that `AudioSource::kind()` in `crates/amp-domain/src/lib.rs` correctly maps HTTP streams to `SourceKind::DirectHttp` instead of `SourceKind::Bandcamp`.
2. Error Standardization: Standardize error enums across `amp-ports` and `amp-domain` (`PortError`, `StreamResolutionError`, `AudioSinkError`, `StorageError`) with explicit error categories rather than ad-hoc strings.
3. Hexagonal Purity: Verify that no domain or port crate imports any concrete adapter or native C library.
4. Comprehensive Unit Tests: Add unit tests verifying exhaustive pattern matching across all `SourceKind` variants and filter bitmasks.

## Verification
```bash
cargo test -p amp-domain -p amp-ports
```

## Acceptance Criteria
- `AudioSource::kind()` correctly maps `DirectHttp` to `SourceKind::DirectHttp`.
- `SourceFilterMask` supports filtering by `DirectHttp`.
- All domain and port tests pass with zero warnings.
- Clean hexagonal boundary separation verified.
