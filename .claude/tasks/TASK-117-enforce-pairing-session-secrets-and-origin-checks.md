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
- `RemoteDaemon` owns pairing tokens (64-bit, single-use, 10 min TTL), session secrets (128-bit, stored as SHA-256 in `~/.multiplay/remote_sessions.json`, revocable via "Forget all paired devices") and a per-IP throttle (5 free failures, then 1 s doubling to 300 s).
- Upgrade checks before the handshake completes: path `/ws`, Host must be an IP literal or `localhost` (DNS-rebinding defence), Origin must equal the daemon origin, peer cap 8 (503), credential (401), throttle (429 + Retry-After).
- 64 KiB message/frame cap, 10 s handshake timeout; HTTP responses send `nosniff`, `no-referrer`, CSP, `X-Frame-Options: DENY`, never CORS.
- Web client: reads `?token=`, strips it from the URL, stores the granted session in localStorage, reconnects with `?session=`.
- Evidence: `cargo test -p amp-adapter-remote --test security` → 15 passed (stable over repeated runs).

## Status
- **Status:** Pending
