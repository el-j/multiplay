# TASK-166: Soulseek UI: Account Settings, Transfer Manager, User Browse

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `frontend_gui`
- Priority: **P2**
- Audit findings: `SRC-16`, `UX-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-164, TASK-165

## Objectives
1. Account login in settings (vault-backed); transfers window (downloads/uploads, queue position, speed, cancel/retry); browse a user's shares from a search result.

## Acceptance Criteria (user-visible / measurable)
- Manual acceptance on GUI; TUI gets a transfers panel.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/soulseek_window.rs`
- `crates/amp-adapter-tui/src/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

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
