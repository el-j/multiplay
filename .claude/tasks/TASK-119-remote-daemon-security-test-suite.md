# TASK-119: Remote Daemon Security Test Suite

## Plan Reference
- Plan: [PLAN-035](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-035-remote-control-security-hardening.md)
- Role: `qa_and_hardening`
- Priority: **P1**
- Audit findings: `SEC-02`, `SEC-03`, `SEC-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-118

## Objectives
1. Integration tests: unauthenticated upgrade rejected; cross-origin rejected; replayed consumed token rejected; oversized frame closes the socket; malformed JSON ignored without panic; path-enqueue attempt rejected.
2. Add the remote JSON decoder as a fuzz target (feeds TASK-185).

## Acceptance Criteria (user-visible / measurable)
- All negative tests pass on 3 OSes in CI.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-remote/tests/security.rs`

## Verification Command
`cargo test -p amp-adapter-remote --test security`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
