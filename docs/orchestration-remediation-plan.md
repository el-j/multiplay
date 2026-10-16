# Multiplay Orchestration Remediation Plan

Date: 2026-10-16
Status: Required before any honest completion claim

## Purpose

This document converts the audit in [docs/orchestration-audit.md](orchestration-audit.md) into a concrete execution plan. The goal is not to “polish” the project; it is to stop over-claiming completion and rebuild from the smallest real, testable, end-to-end system.

## Working principle

The project must stop treating orchestration metadata as completion proof. The only valid completion signal is fresh evidence:

- clean workspace build
- full passing test suite
- real end-to-end playback validation
- actual provider implementations or explicit, documented exclusions
- complete rustdoc coverage
- no placeholder or generated-data fallback paths in production code
- no unresolved warnings or type issues

## Hard gating rules

Before any task may be marked complete, all of the following must be true:

1. The specific feature has a real test proving the behavior.
2. The feature is exercised through the real runtime path, not only a unit mock.
3. No synthetic fallback is left in production behavior unless explicitly documented as a temporary development-only mode.
4. The change is reflected in the task and plan metadata honestly.
5. Verification is rerun from a clean checkout state.

## Delivery strategy

### Phase 0 — Truthful baseline and scope reduction

Goal: stop claiming completion before we have a valid system.

Priority: P0

Tasks:

- Freeze the current roadmap metadata and mark it as aspirational rather than complete.
- Inventory every task that claims completion but has no live verification evidence.
- Remove or downgrade fake “complete” claims from the orchestrator metadata.
- Identify and explicitly classify the current system as “prototype / architecture-in-progress”.
- Reduce scope to the smallest viable product slice:
  - a local library playback path
  - a real decode pipeline
  - a working output path
  - a real search-provider interface

Acceptance criteria:

- The orchestrator no longer says “completed” for features without evidence.
- The project has a clear, honest status label.
- The roadmap priorities are re-ranked against actual risk and viability.

Files and hotspots:

- [.claude/orchestrator-index.md](../.claude/orchestrator-index.md)
- [.claude/orchestrator.json](../.claude/orchestrator.json)
- [docs/orchestration-audit.md](orchestration-audit.md)

---

### Phase 1 — Replace placeholder providers with real behavior or explicit exclusions

Goal: eliminate synthetic provider behavior from the system.

Priority: P0

#### 1.1 Spotify adapter

Target file:

- [crates/amp-adapter-spotify/src/lib.rs](../crates/amp-adapter-spotify/src/lib.rs)

Required action:

- Remove the generated WAV fallback as a production code path.
- Either implement a genuine Spotify-compatible resolver contract or clearly gate the adapter as a non-production stub with explicit developer-only configuration.
- Add tests for authentication failure, empty source handling, and valid live-path behavior.

Acceptance criteria:

- No production logic synthesizes audio in place of a provider result.
- All live-data branches are tested.
- Fallback behavior is either removed or quarantined to a debugging-only mode.

#### 1.2 Soulseek adapter

Target file:

- [crates/amp-adapter-soulseek/src/lib.rs](../crates/amp-adapter-soulseek/src/lib.rs)

Required action:

- Remove hard-coded “Aphex” search behavior and generated audio responses.
- Define a minimal real network contract or a strict no-live-network mode with explicit behavior.
- Add deterministic tests for packet validation and error paths.

Acceptance criteria:

- Query behavior is not keyed to one string or one metadata artifact.
- Packet parsing remains valid and safe.
- Any network-dependent path is behind clear fail-fast handling.

#### 1.3 P2P mesh adapter

Target file:

- [crates/amp-adapter-p2p/src/lib.rs](../crates/amp-adapter-p2p/src/lib.rs)

Required action:

- Remove fabricated “P2P Mesh Anthem” results and WAV generation fallback.
- Make the peer model explicit: discovery, trust, and transport behavior must be coded as deliberate contracts, not placeholder logic.
- Add tests for trusted and untrusted peers, empty payloads, and missing data behavior.

Acceptance criteria:

- No fabricated music content is generated in the adapter.
- The peer trust contract is tested end-to-end at the adapter boundary.
- Missing or untrusted payloads return precise errors.

---

### Phase 2 — Establish a real runtime pipeline

Goal: prove the engine actually carries a track from source to hardware output.

Priority: P0

Targets:

- [crates/amp-app/src/main.rs](../crates/amp-app/src/main.rs)
- [crates/amp-adapter-cpal/src/lib.rs](../crates/amp-adapter-cpal/src/lib.rs)
- [crates/amp-adapter-decoder/src/lib.rs](../crates/amp-adapter-decoder/src/lib.rs)
- [crates/amp-core-engine/src/lib.rs](../crates/amp-core-engine/src/lib.rs)
- [crates/amp-domain/src/lib.rs](../crates/amp-domain/src/lib.rs)

Required actions:

- Choose a real playback path that works on the current host, with a minimal but valid source set.
- Wire the decode worker to the audio sink using a verified ring-buffer contract.
- Replace “sample track only” runtime behavior with a real playback loop that can accept a validated track source.
- Add integration tests around the actual runtime path.

Acceptance criteria:

- A track can move from source descriptor to decoded buffer to output sink without placeholder logic.
- Playback state changes are observable and testable.
- The runtime is demonstrably connected, not just assembled in memory.

---

### Phase 3 — Repair the domain contracts and cross-crate consistency

Goal: remove mismatches between architecture and code.

Priority: P1

Targets:

- [crates/amp-domain/src/lib.rs](../crates/amp-domain/src/lib.rs)
- [crates/amp-domain/src/source_badge.rs](../crates/amp-domain/src/source_badge.rs)
- [crates/amp-ports/src/lib.rs](../crates/amp-ports/src/lib.rs)
- [crates/amp-core-engine/src/filtered_search.rs](../crates/amp-core-engine/src/filtered_search.rs)

Required actions:

- Fix inconsistent source mapping such as `DirectHttp` vs `SourceKind` handling.
- Make `SourceFilterMask`, `AudioSource`, and provider contracts consistent across the domain and ports.
- Standardize error semantics across provider and stream resolution code.
- Remove stale references to unsupported or non-implemented capabilities.

Acceptance criteria:

- All source enums and badges agree on all variants.
- A provider contract is consistent across adapters and search logic.
- Search and stream resolution are validated against the same source model.

---

### Phase 4 — Fix tests and close coverage gaps

Goal: replace weak unit coverage with honest, behavior-focused validation.

Priority: P0

Required actions:

- Write failing tests before implementation for each fixed behavior.
- Add end-to-end tests for:
  - local file playback
  - source selection and playback state changes
  - provider resolution failures
  - audio-sink control path
  - search coordinator and timeout behavior
- Add negative tests for all fail-fast branches.
- Add stress tests for queueing, shuffle, and ring-buffer pressure.

Acceptance criteria:

- Full workspace tests pass reliably.
- There is evidence that critical runtime paths are tested as real behavior, not just mocks.
- No behavior is only verified by a synthetic stand-in.

Targets to prioritize:

- [crates/amp-adapter-storage/src/lib.rs](../crates/amp-adapter-storage/src/lib.rs)
- [crates/amp-core-engine/src/filtered_search.rs](../crates/amp-core-engine/src/filtered_search.rs)
- [crates/amp-core-engine/src/playback/shuffle.rs](../crates/amp-core-engine/src/playback/shuffle.rs)
- [crates/amp-adapter-cpal/src/lib.rs](../crates/amp-adapter-cpal/src/lib.rs)
- [crates/amp-app/src/main.rs](../crates/amp-app/src/main.rs)

---

### Phase 5 — Mutation testing and static health enforcement

Goal: prove the system is robust rather than merely compiling.

Priority: P1

Required actions:

- Execute mutation testing with the existing configuration in [.cargo/mutants.toml](../.cargo/mutants.toml).
- Fix or remove any test suite gaps revealed by mutants.
- Run clippy and treat warnings as failing conditions.
- Run a full verification command that reflects the repository’s actual contract.

Acceptance criteria:

- Mutation testing has real output and no unresolved gaps.
- Warning-free checks are enforced.
- The project can honestly claim health, not just compilation.

---

### Phase 6 — Documentation and code hygiene

Goal: make the code readable by humans and the docs truthful.

Priority: P1

Targets:

- [crates/amp-app/src/main.rs](../crates/amp-app/src/main.rs)
- [crates/amp-adapter-gui/src/app.rs](../crates/amp-adapter-gui/src/app.rs)
- [crates/amp-adapter-cpal/src/lib.rs](../crates/amp-adapter-cpal/src/lib.rs)
- [crates/amp-adapter-decoder/src/lib.rs](../crates/amp-adapter-decoder/src/lib.rs)

Required actions:

- Split large boss-files into smaller focused modules.
- Add rustdoc comments to every public symbol, and make internal behavior clearer.
- Replace vague comments with purpose and contract statements.
- Remove stale references and generated-data assumptions from docs.

Acceptance criteria:

- Large modules are reduced or clearly separated by concern.
- Documentation matches the code and the product reality.
- New contributors can reason about the runtime path without reading 400+ lines in one file.

---

## Recommended order of execution

1. Truthful status freeze and scope reduction
2. Remove synthetic provider behavior in Spotify / Soulseek / P2P
3. Validate a minimal real playback path end-to-end
4. Fix contracts and source model inconsistencies
5. Add missing e2e and negative tests
6. Run mutation and warning gates
7. Revisit documentation and boss-file reduction
8. Only then update orchestrator status to complete

## Risk notes

The highest-risk issue is not “missing polish.” It is the false perception of completeness created by the orchestration metadata itself. Until the project removes synthetic provider behavior and proves the real runtime path, the roadmap is not a reliable completion signal.

## Exit condition for closure

The project may only be closed when all of the following are true:

- real runtime playback works in a verified flow
- provider adapters either work or are clearly excluded with explicit documentation
- the full test suite passes cleanly
- mutation testing has been run and gaps are closed
- warnings and type issues are eliminated
- documentation matches implementation
- the orchestrator metadata reflects reality rather than ambition

Until that point, the project should remain in a design/prototype state, not a completed delivery state.
