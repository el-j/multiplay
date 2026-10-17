# TASK-162: Soulseek Server Session: Login, Keepalive, Reconnect, Correct Message Namespaces

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-13` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-151

## Objectives
1. Separate codecs for server, peer-init, peer and distributed message namespaces with bounded frame lengths.
2. Login (username, password, version, MD5(username+password), minor version), SetWaitPort, SetStatus, ping, reconnect with backoff; credentials from vault.
3. Async actor (`tokio::net`) exposing a typed command/event API.

## Acceptance Criteria (user-visible / measurable)
- Login against the real server with a test account succeeds; codec round-trip + fuzz tests pass.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soulseek/src/{codec,server,session}.rs`

## Verification Command
`cargo test -p amp-adapter-soulseek`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
