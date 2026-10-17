# TASK-134: Honest AudioSinkPort: Remove Dead push API, Real configure, Correct Latency

## Plan Reference
- Plan: [PLAN-038](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-038-audio-output-device-robustness.md)
- Role: `architecture`
- Priority: **P2**
- Audit findings: `AUD-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Redesign `AudioSinkPort` around the real data path (producer handle + control), delete `push_interleaved_samples` stub.
2. `configure` actually reopens the stream or is removed; `latency()` computed from frames, rate and device-reported buffer.

## Acceptance Criteria (user-visible / measurable)
- No port method returns constant/fake values.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-ports/src/driven.rs`
- `crates/amp-adapter-cpal/src/lib.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
