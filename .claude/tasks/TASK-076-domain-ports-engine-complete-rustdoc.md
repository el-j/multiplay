---
id: TASK-076
title: Domain, Ports & Core Engine Complete Rustdoc Documentation
planId: PLAN-025
status: completed
role: documentation
verificationCommand: cargo doc -p amp-domain -p amp-ports -p amp-core-engine --no-deps
---

# TASK-076: Domain, Ports & Core Engine Complete Rustdoc Documentation

## Description
Document 100% of public and internal APIs across the core domain and engine crates (`docs/orchestration-audit.md` section 2 & DoD):
1. `amp-domain`: Add comprehensive rustdoc to all entities (`Track`, `Playlist`, `StreamSource`, `AudioMeta`), value objects, and badge enums (`SourceKind`, `SourceFilterMask`), documenting contracts, invariants, and serialization formats.
2. `amp-ports`: Document all driving and driven port traits (`PlayerControlPort`, `AudioSinkPort`, `StreamResolverPort`, `LibraryStoragePort`), explicitly documenting real-time thread safety requirements, callback semantics, and error behaviors.
3. `amp-core-engine`: Document all DSP filters (biquad math, graphic equalizer, soft-knee limiter curve, EBU R128 loudness calculation), precache buffers, shuffle logic, and CQRS dispatchers with mathematical formulas and architectural rationales.
4. Clean Doc Tests: Ensure doc examples compile and run cleanly with `cargo test --doc`.

## Verification
```bash
cargo doc -p amp-domain -p amp-ports -p amp-core-engine --no-deps
```

## Acceptance Criteria
- 100% rustdoc coverage across `amp-domain`, `amp-ports`, and `amp-core-engine`.
- All doc tests pass cleanly.
- Clear mathematical and real-time safety contracts documented on all DSP algorithms.
