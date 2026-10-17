# TASK-130: Typed Port Errors & User-Visible Error Events

## Plan Reference
- Plan: [PLAN-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-037-unified-core-playback-engine.md)
- Role: `architecture`
- Priority: **P2**
- Audit findings: `ARC-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Adopt `amp-ports::error` types on every port (replace `Result<_, String>`); remove all `e.contains("WouldBlock")` string matching.
2. `PlayerEvent::Error { kind, source, message, recoverable }` emitted by the engine; UIs show them (TASK-175).

## Acceptance Criteria (user-visible / measurable)
- `grep -rn 'Result<.*String>' crates/amp-ports` returns nothing.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-ports/src/`
- `crates/amp-core-engine/src/events.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
