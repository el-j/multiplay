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
- `crates/amp-adapter-remote/tests/security.rs` (15 tests): unauthenticated/unknown/expired token → 401; replayed token → 401; session reconnect then revoke → 401; cross-origin → 403; DNS-rebinding Host → 403; throttling → 429 (even for a valid token while backed off); peer cap → 503; 100 KiB frame closes the socket with no command dispatched; malformed/unversioned JSON answered with Error and the session survives; path/URL enqueue refused; HTTP security headers; shutdown closes the listener.
- Fuzz harness `crates/amp-adapter-remote/fuzz` (`cargo +nightly fuzz run remote_protocol`), kept outside the workspace because libfuzzer-sys builds C++. **Not yet run** (no nightly toolchain locally); a 5,000-case deterministic no-panic test runs in normal CI.

## Status
- **Status:** Pending
