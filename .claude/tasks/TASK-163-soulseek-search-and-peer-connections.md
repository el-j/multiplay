# TASK-163: Soulseek Search & Peer Connections (Direct + Firewalled)

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-14` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-162

## Objectives
1. FileSearch with token; accept incoming peer connections on listen port; outgoing direct connect with fallback to ConnectToPeer/PierceFirewall.
2. Parse zlib-compressed FileSearchResponse incl. attributes (bitrate, duration, VBR, sample rate, bit depth); stream results to the search coordinator.

## Acceptance Criteria (user-visible / measurable)
- Searching a common query returns ≥ 50 results within 10 s on a real network (manual).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soulseek/src/{peer,search}.rs`

## Verification Command
`cargo test -p amp-adapter-soulseek search`

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
