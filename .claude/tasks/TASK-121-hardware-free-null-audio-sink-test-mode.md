# TASK-121: Hardware-Free Test Mode with Null/Capture Audio Sink

## Plan Reference
- Plan: [PLAN-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-036-green-ci-and-hermetic-tests.md)
- Role: `qa_and_hardening`
- Priority: **P0**
- Audit findings: `QA-01` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Introduce `NullAudioSink` and `CaptureAudioSink` (in a `amp-test-support` dev crate) implementing the sink port.
2. All app/GUI/engine tests use injected sinks; tests that need hardware get `#[ignore = "needs audio hardware"]` and a `make test-hw` target.

## Acceptance Criteria (user-visible / measurable)
- `cargo test --workspace` passes on a machine with no audio devices (CI Linux).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-test-support/`
- `crates/amp-adapter-cpal/src/lib.rs`
- `Makefile`

## Verification Command
`cargo test --workspace`

## Manual Verification
- `CpalAudioSink::new_null(rate, channels)`: drains the ring buffer at real-time pace on a background thread, no device opened. `AudioPlaybackDriver::new_headless()` in GUI and TUI; `MultiplayGuiApp::headless(root)` and `MultiplayApp::new_headless()` used by all app tests. The streaming e2e test previously skipped itself silently without a device; it now always runs.
- The 3 hardware tests in `amp-adapter-cpal` are `#[ignore = "needs audio hardware"]`; `make test-hw` runs them.
- Deviation: the null sink lives in `amp-adapter-cpal` (also needed for a headless mode, BLD-07) instead of a separate `amp-test-support` crate; a `CaptureAudioSink` for asserting output samples is still open.

## Status
- **Status:** Pending
