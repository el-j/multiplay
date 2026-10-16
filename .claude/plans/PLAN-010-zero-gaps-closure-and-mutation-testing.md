---
id: PLAN-010
title: Zero-Gaps Closure, Live Playback Streaming & 100% Mutation Testing
status: completed
role: qa_and_hardening
tasks:
  - TASK-029
  - TASK-030
  - TASK-031
  - TASK-032
---

# PLAN-010: Zero-Gaps Closure, Live Playback Streaming & 100% Mutation Testing

## Objective
Address every gap identified in the comprehensive technical audit to achieve 100% Definition of Done: connect dynamic audio decoding worker threads to CPAL in the runtime binary, replace simulated/draft network stream cursors with live streaming channels, enforce 100% in-code documentation across all crates, and configure property-based testing and mutation testing matrices.

## Architecture Context
- Vision Docs: `docs/vision/real_time_audio_engine_cpal_spsc_ring_buffer.md`, `docs/vision/streaming_sources_multi_source_orchestration.md`.
- Target Areas:
  - `crates/amp-app/src/main.rs`: Dynamic audio thread bridging `StreamResolverPort` to `CpalAudioSink`.
  - `crates/amp-adapter-spotify`, `crates/amp-adapter-soulseek`, `crates/amp-adapter-p2p`: Live stream resolver channels.
  - Property testing via `proptest` for DSP filter stability.
  - Full mutation test coverage verification via `cargo-mutants`.

## Tasks
1. [TASK-029](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-029-runtime-audio-playback-streaming-worker.md): Dynamic Background Audio Playback Streamer in Runtime Composition Root
2. [TASK-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-030-live-protocol-stream-resolvers.md): Live Protocol Stream Handlers for Spotify, Soulseek TCP, and Encrypted P2P
3. [TASK-031](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-031-property-testing-and-mutation-suite.md): Comprehensive Property-Based Testing (`proptest`) & Mutation Test Suite
4. [TASK-032](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-032-complete-incode-docstrings-and-audit-closure.md): 100% In-Code Documentation & Architecture Reference Alignment Audit

## Definition of Done
- Runtime binary plays audio into hardware CPAL output upon selecting any track.
- Zero mock or placeholder stream cursors remain.
- All public and internal types, structs, traits, and functions contain complete, verified rustdoc docstrings.
- 100% pure Rust verified with zero warnings and passing mutation tests.
