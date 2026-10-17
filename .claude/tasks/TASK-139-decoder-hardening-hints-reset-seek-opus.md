# TASK-139: Decoder Hardening: Probe Hints, ResetRequired, Accurate Seek, Opus/WebM

## Plan Reference
- Plan: [PLAN-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-039-decoder-and-progressive-streaming-correctness.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: `AUD-11`, `AUD-12`, `AUD-13` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Pass extension/MIME `Hint`s from source and resolver.
2. Rebuild decoder on `ResetRequired` (chained Ogg).
3. Seek via `SeekTo::Time`, use returned `actual_ts`, trim pre-roll for sample accuracy; `Accurate` mode for local files.
4. Enable `mkv` demuxer and add an Opus decode path (pure-Rust crate) — or remove `.opus` from scanner/dialog/docs with an ADR.

## Acceptance Criteria (user-visible / measurable)
- Fixture tests: chained Vorbis plays across boundaries; seek accuracy ±1 frame on FLAC/WAV; Opus either plays or is cleanly rejected and not advertised.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-decoder/src/stream.rs`
- `crates/amp-adapter-decoder/Cargo.toml`

## Verification Command
`cargo test -p amp-adapter-decoder`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
