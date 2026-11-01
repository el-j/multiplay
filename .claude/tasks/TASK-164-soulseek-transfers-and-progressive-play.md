# TASK-164: Soulseek Transfers: Queue, F-Connection, Resume & Progressive Play

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-15` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-163, TASK-135

## Objectives
1. QueueUpload, TransferRequest/Response, PlaceInQueue updates, F connection with offset; resume partial files from the cache dir.
2. `resolve_stream` returns a progressive precache stream fed by the transfer; completed files optionally kept in downloads/library.

## Acceptance Criteria (user-visible / measurable)
- Play-while-downloading of a FLAC from a real peer works; interrupted transfer resumes.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soulseek/src/transfer.rs`

## Verification Command
`cargo test -p amp-adapter-soulseek transfer`

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
