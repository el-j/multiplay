# TASK-117: Enforce Pairing, Session Secrets, Origin Allow-List & Rate Limits

## Plan Reference
- Plan: [PLAN-035](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-035-remote-control-security-hardening.md)
- Role: `security`
- Priority: **P0**
- Audit findings: `SEC-02`, `SEC-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-116

## Objectives
1. Move `PairingManager` into `RemoteDaemon`; reject WebSocket upgrades without a valid pairing token or session secret (HTTP 401 before upgrade).
2. On successful pairing, issue a 128-bit random session secret (stored hashed host-side, persisted, revocable from settings).
3. Reject upgrades whose `Origin` is not the daemon's own origin; add `X-Content-Type-Options`, no CORS.
4. Throttle failed attempts per IP (exponential backoff), cap concurrent connections (e.g. 8), cap frame size (64 KB).

## Acceptance Criteria (user-visible / measurable)
- A page on another origin cannot open a session.
- Wrong/expired tokens are rejected; 100 rapid bad attempts are throttled.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-remote/src/server.rs`
- `crates/amp-adapter-remote/src/pairing.rs`
- `crates/amp-adapter-remote/src/web_client.rs`

## Verification Command
`cargo test -p amp-adapter-remote && cargo test -p amp-app --test remote_daemon_pipeline`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
