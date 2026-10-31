# TASK-193: Privacy Statement & Network Activity Inventory

## Plan Reference
- Plan: [PLAN-052](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-052-legal-privacy-and-provider-compliance.md)
- Role: `security`
- Priority: **P2**
- Audit findings: `LEG-04`, `SEC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-116

## Objectives
1. Document every outbound/inbound connection, what is sent, and when; no telemetry; accurate User-Agent with real version.

## Acceptance Criteria (user-visible / measurable)
- `docs/privacy.md` linked from README and About; matches code (reviewed against `grep -rn reqwest|TcpListener|UdpSocket`).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `docs/privacy.md`
- `crates/amp-adapter-http/src/lib.rs`

## Verification Command
`test -f docs/privacy.md`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31, branch feat/beta-release-polish)
- `docs/privacy.md`: every connection (YouTube, Bandcamp, streams, About links), remote-control defaults (off, 127.0.0.1:9002, LAN opt-in), stored files, Soulseek/P2P not connected yet. Linked from README and Preferences → About.
- Guard test `crates/amp-app/tests/privacy_statement.rs`: every host in a production `const` must appear in the statement; remote defaults and "No telemetry" checked.
- User-Agent: `amp_domain::about::user_agent()` = `multiplay/<version> (+https://github.com/el-j/multiplay)` for amp-adapter-http and Bandcamp (was a browser-style string); `crates/amp-domain/tests/about.rs`. Live Bandcamp tests (`--ignored`) pass with it. YouTube requests still use rusty_ytdl's own headers (documented).
