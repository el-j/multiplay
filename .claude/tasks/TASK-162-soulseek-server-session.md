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

## Implementation Evidence (2026-11-01, branch feat/drop-spotify-soundcloud) — re-scoped
- Product decision: no own Soulseek protocol client. "Use Soulseek" goes through slskd (ADR-008): connect to the user's slskd, or install the official release on request and run it.
- `crates/amp-adapter-soulseek`: `slskd::SlskdClient` (version, options, searches, download batches with explicit destination, transfer state), `SoulseekProvider` (search + stream resolver, 10-minute download timeout via the new `StreamResolverPort::resolve_timeout`), `install` (slskd 0.26.2 per OS, pinned SHA-256, zip-slip checks), `daemon` (127.0.0.1 only, generated API key, no shares, settings in the environment, stopped on drop).
- App: Preferences → P2P → Soulseek (off by default; managed or own slskd; secrets in the encrypted vault); search includes Soulseek only once connected; the TUI connects to the same slskd when enabled.
- Tests: `amp-adapter-soulseek/tests/slskd.rs` (fake slskd: detection, wrong key, search filtering/ordering, download + play, failure reason), `tests/install.rs` (pinned assets, checksum, zip slip, environment, stop), `amp-adapter-gui/tests/soulseek_setup.rs`.
- Not verified: against a real slskd with a real Soulseek account (needs the user's account).
