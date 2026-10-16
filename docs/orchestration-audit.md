# Multiplay Orchestration Audit

Date: 2026-10-16
Status: Not compliant with the repository Definition of Done

## Scope and method

This audit reviewed the orchestration metadata and the code under the workspace:

- [.claude/orchestrator-index.md](../.claude/orchestrator-index.md)
- [.claude/orchestrator.json](../.claude/orchestrator.json)
- [.claude/plans](../.claude/plans)
- [.claude/tasks](../.claude/tasks)
- [Cargo.toml](../Cargo.toml)
- [crates](../crates)
- [scripts/verify_pure_rust.sh](../scripts/verify_pure_rust.sh)
- [.cargo/mutants.toml](../.cargo/mutants.toml)

The review was practical and code-grounded: it compared the roadmap claims against the actual Rust implementation, build behavior, and test execution in this workspace.

## Executive verdict

The project is not ready to claim 100% completion against its own Definition of Done. The orchestration system says 19 of 19 plans are complete and 60 of 60 tasks are complete, but the actual workspace does not support that claim. The codebase is best described as an ambitious, partially implemented prototype with a strong roadmap and a lot of scaffolding, not a fully verified production-grade audio system.

This is not a failure of ambition; it is a failure of verification and closure. The project has promising architecture and a clear roadmap, but the implementation has a significant gap between the written plan and the real behavior.

## Definition-of-Done check against reality

| Criterion | Actual status | Evidence |
| --- | --- | --- |
| 100% functional | Not met | Real providers are synthetic and not wired to real live media pipelines. |
| No design issues | Not met | Roadmap claims are broader than the implementation; architecture is aspirational but not fully connected. |
| 100% test coverage | Not met | Full workspace tests are not passing in this environment; the suite was killed by the OS. |
| No gaps or stub code | Not met | Synthetic audio streams and hard-coded results remain in providers. |
| No placeholders | Not met | Generated PCM WAVs, fixed metadata, and mocked peer behavior are present. |
| 100% documented correctly | Not met | The task backlog states full docs, but the code does not show complete rustdoc consistency or verified doc generation. |
| 100% in-code documentation | Not met | Public and internal code is unevenly documented; many modules are large and not self-explanatory. |
| 100% e2e tested | Not met | No evidence of end-to-end validation of the real audio pipeline in this workspace. |
| 100% mutation tested | Not met | A config exists, but no mutation-test execution is evidenced here. |
| No code warnings | Not met | Not proven by the workspace verification; warnings were not validated as clean. |
| No type issues | Not met | Not proven by a clean full workspace pass; compile/test evidence is incomplete. |

## 1) Orchestration-system integrity: roadmap claims do not match reality

The orchestration metadata in [.claude/orchestrator-index.md](../.claude/orchestrator-index.md) and [.claude/orchestrator.json](../.claude/orchestrator.json) declares a fully completed project:

- 19 plans complete
- 60 tasks complete
- status: completed
- verification command: `./scripts/verify_pure_rust.sh && cargo test --workspace`

But the actual workspace evidence does not validate those claims:

- `cargo test --workspace --quiet` was killed by the environment with exit code 137.
- `cargo test -p amp-domain -p amp-ports -p amp-core-engine -p amp-adapter-storage -p amp-adapter-tui -p amp-adapter-gui -p amp-app --quiet` also killed with exit code 137.
- `cargo check --workspace --quiet` emitted no compiler errors in this session, but a compile check is not the same as a fully proven end-to-end, tested, and documented system.

The orchestration system is therefore acting like a project tracker and completion state machine rather than a verified source of truth. It is aspirational metadata, not evidence-backed completion status.

## 2) Missing or wrong inline documentation and stale implementation references

The project claims complete in-code documentation in the task set, but the code does not consistently show that. Several obvious mismatches exist:

- [crates/amp-domain/src/lib.rs](../crates/amp-domain/src/lib.rs): `AudioSource::kind()` maps `DirectHttp` to `SourceKind::Bandcamp`, even though `SourceKind` in [crates/amp-domain/src/source_badge.rs](../crates/amp-domain/src/source_badge.rs) does not define a `DirectHttp` variant. This is an implementation bug and a documentation mismatch.
- The code comments and docstrings are uneven: several files have doc comments, but many large implementation modules are still effectively monolithic and hard to reason about without reading long stretches of logic.
- The task docs describe a highly sophisticated implementation, but the code in the provider adapters shows a much simpler synthetic model than the design language suggests.

This means the documentation is not “wrong” in every line, but it is not fully aligned with the code reality.

## 3) Gaps in implementation: the architecture is larger than the working system

The biggest gap is that most provider-specific crates are not live integrations; they are synthetic adapters.

### 3.1 Spotify adapter is synthetic, not a real integrated provider

See [crates/amp-adapter-spotify/src/lib.rs](../crates/amp-adapter-spotify/src/lib.rs).

Evidence:

- `PureSpotifyAdapter::resolve_stream()` checks authentication and then returns generated PCM WAV data if no preloaded data is cached.
- The code does not call a real Spotify API or real protocol stack.
- The implementation is a local in-memory stream generator, not a live provider implementation.

This is not a legitimate provider integration; it is a stand-in.

### 3.2 P2P mesh is not a real encrypted mesh implementation

See [crates/amp-adapter-p2p/src/lib.rs](../crates/amp-adapter-p2p/src/lib.rs).

Evidence:

- `SecurePeerNode::query()` returns a hard-coded result for a peer-based track named “P2P Mesh Anthem”.
- `resolve_stream()` returns generated PCM WAV bytes when the file hash is missing.
- Trust is only checked in-memory; there is no real encrypted transport, handshake, or network discovery path.

This is a valid mock/test harness, but it is not a true production P2P system.

### 3.3 Soulseek is protocol-shaped but not a real network client

See [crates/amp-adapter-soulseek/src/lib.rs](../crates/amp-adapter-soulseek/src/lib.rs).

Evidence:

- Packet encode/decode functions exist and are simple enough to pass unit tests.
- Query logic only returns “Aphex” results if the query contains “aphex”.
- Stream resolution falls back to a generated WAV byte vector instead of a live peer transfer path.

This is useful as a protocol skeleton, but it is not a live transfer implementation.

### 3.4 Runtime composition root is not a fully connected end-to-end pipeline

See [crates/amp-app/src/main.rs](../crates/amp-app/src/main.rs).

Evidence:

- The app creates sample tracks and pushes them into UI state.
- It handles keyboard inputs and toggles playback state.
- It does not demonstrate a complete pipeline from provider stream resolution to decoder, DSP, ring-buffer management, and CPAL output in a truly connected runtime execution path.

This is enough to show an app shell and local UI control loop, but not enough to prove the full audio engine is working end-to-end.

## 4) Missing tests and no validated coverage story

There are tests in several crates, but the total project verification story is incomplete.

Examples:

- [crates/amp-domain/src/lib.rs](../crates/amp-domain/src/lib.rs) has unit tests.
- [crates/amp-adapter-storage/src/lib.rs](../crates/amp-adapter-storage/src/lib.rs) has a persistence round-trip test.
- [crates/amp-core-engine/src/filtered_search.rs](../crates/amp-core-engine/src/filtered_search.rs) has search tests.
- [crates/amp-core-engine/src/playback/shuffle.rs](../crates/amp-core-engine/src/playback/shuffle.rs) has shuffle tests.
- [crates/amp-adapter-spotify/src/lib.rs](../crates/amp-adapter-spotify/src/lib.rs) has unit tests for authentication and stream generation.

But:

- Full workspace tests are not passing in this environment.
- No evidence shows real end-to-end tests covering live streaming, sleep-free CPAL playback, network decoders, real provider APIs, or remote UI integration.
- The mutation configuration exists in [.cargo/mutants.toml](../.cargo/mutants.toml), but there is no evidence that it was executed successfully.

This is a classic case of “unit tests exist, but not enough to prove the system is production-ready.”

## 5) Boss-files and oversized modules

The repo contains several large single files that are hard to maintain and review.

Large examples by line count:

- [crates/amp-app/src/main.rs](../crates/amp-app/src/main.rs) — 477 lines
- [crates/amp-adapter-gui/src/app.rs](../crates/amp-adapter-gui/src/app.rs) — 447 lines
- [crates/amp-adapter-cpal/src/lib.rs](../crates/amp-adapter-cpal/src/lib.rs) — 407 lines
- [crates/amp-adapter-decoder/src/lib.rs](../crates/amp-adapter-decoder/src/lib.rs) — 388 lines
- [crates/amp-adapter-gui/src/docking.rs](../crates/amp-adapter-gui/src/docking.rs) — 349 lines

These are not necessarily invalid, but they are exactly the kind of boss-files the user warning called out: huge, human-unreadable, and likely to hide design drift and incompleteness. They also contradict the preference for modular, small, focused files.

## 6) Stub code and placeholder behavior

The project contains code that is clearly synthesized for demonstration rather than complete implementation.

Examples:

- [crates/amp-adapter-spotify/src/lib.rs](../crates/amp-adapter-spotify/src/lib.rs): generated WAV fallback instead of a real Spotify stream.
- [crates/amp-adapter-p2p/src/lib.rs](../crates/amp-adapter-p2p/src/lib.rs): fabricated track names and generated payloads instead of encrypted mesh data.
- [crates/amp-adapter-soulseek/src/lib.rs](../crates/amp-adapter-soulseek/src/lib.rs): logic keyed to a single hard-coded string “aphex” and code-generated audio bytes.
- [crates/amp-core-engine/src/playback/shuffle.rs](../crates/amp-core-engine/src/playback/shuffle.rs): contains a direct panic in a test guard, which is a valid test check but not a strong sign of production-hardening.

The grep for `TODO`, `FIXME`, `unimplemented!`, `todo!`, and placeholder markers was not broad enough to find a huge list of explicit stubs, but the runtime behavior itself is still placeholder-like in meaningful ways.

## 7) Todo items and outstanding implementation work

The roadmap is very broad, and the project has not closed enough of that scope to claim completion. The biggest outstanding items are not small polish tasks; they are foundational system behaviors:

- live provider integration for Spotify, Soulseek, and P2P
- real network transport / authentication / encryption
- actual runtime connection between providers, decoder, DSP, and output
- e2e audio playback verification
- full mutation-test execution and failure cleanup
- cross-crate integration validation

In other words, the missing work is not just “clean up state”; it is the core of the product.

## 8) Missing features and unlinked integrations

The architecture claims many features, but several are not materially connected:

- YouTube, Spotify, Soulseek, and P2P source resolvers exist as adapters, but real live streams are not materially connected to the runtime engine.
- The GUI and TUI layers appear to exist as separate visual systems, but the integration path to a single coherent runtime is not robustly validated.
- Search filtering and provider registration exist, but the actual provider pipeline is not fully integrated with true remote media fetching.
- DJ, DSP, and equalizer modules exist, but the end-to-end playback chain is not proven end-to-end.

The result is an architecture that looks complete on paper and in module names, but not complete in behavior.

## 9) E2E untested features

The highest-risk features are the ones that are described as “complete” in the orchestration system but are not proven in end-to-end execution:

- real audio decode and playback path
- live provider search and stream resolution
- audio ring-buffer flow to hardware output
- multi-deck or DJ coordination path
- TUI or GUI interaction through real playback state transitions
- remote or mesh synchronization features
- cross-platform packaging and launcher behavior

There is no proof that these flows work together in a real user scenario. The workspace contains unit tests, but not robust e2e proof for the actual product as shipped.

## 10) Honest conclusion

This repository should be classified as a strong prototype or architecture-in-progress, not a completed product.

The orchestration system is over-claiming completion. The project has:

- a thoughtful modular architecture
- a large and well-structured roadmap
- meaningful starter implementations across the major crates
- some valuable unit tests

But it does not yet meet the repository’s own Definition of Done:

- not 100% functional
- not fully connected end-to-end
- not fully verified by tests
- not fully mutation-tested
- not fully documented and aligned with reality
- not free of placeholder behavior
- not fully proven across real user-facing scenarios

### Final verdict

Status: Not complete. Needs a real re-baselining and hard verification before it can honestly be called finished.

This is not a failed project; it is a project with a good concept and an incomplete production implementation. The next step is not cosmetic cleanup. The next step is to reduce scope, validate the real end-to-end audio pipeline, replace synthetic provider behavior with real integrations, and then prove the whole system via a clean test and documentation pass.
