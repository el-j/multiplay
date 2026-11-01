# TASK-165: Soulseek Sharing: Share Index, Browse/Search Responses, Upload Slots & Limits

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `distributed_systems`
- Priority: **P1**
- Audit findings: `SRC-16` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-163, TASK-149

## Objectives
1. Build share index from `shared_directories` (from library DB); respond to search and browse requests; upload queue with `max_upload_slots` and bandwidth caps.
2. Ignore/ban list; share-count reporting; privacy: never share outside configured folders (path canonicalization tests).

## Acceptance Criteria (user-visible / measurable)
- Another client (e.g. Nicotine+) can browse and download from multiplay.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soulseek/src/share.rs`

## Verification Command
`cargo test -p amp-adapter-soulseek share`

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
