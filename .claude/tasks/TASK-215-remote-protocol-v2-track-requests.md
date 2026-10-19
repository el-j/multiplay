# TASK-215: Remote Protocol v2: RequestTrack with Allow-Listed References

## Plan Reference
- Plan: [PLAN-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-056-remote-and-sharing-window-host-moderated-requests.md)
- Design: [Signature features design §7](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `security`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-118, TASK-119

## Objectives
1. `RequestTrack { reference }` where reference = library TrackId | allow-listed provider link (YouTube, Bandcamp, SoundCloud as resolvers exist) | search query.
2. Server validation: URL parsing with host allow-list, no file paths, length limits, per-device rate limit and pending-request cap; requests never auto-play.
3. Request lifecycle events to phones: pending, accepted, rejected, playing.
4. Extend `tests/security.rs` and the fuzz target for the new messages.

## Acceptance Criteria (user-visible / measurable)
- Negative tests: file path, `file://`, private-IP URL, unknown host, oversize payload and flood are all rejected.
- Policy text in SECURITY.md/README updated: 'remotes never make the host open anything without approval'.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-remote/src/protocol.rs`
- `crates/amp-adapter-remote/src/server.rs`
- `crates/amp-adapter-remote/tests/security.rs`
- `crates/amp-adapter-remote/fuzz/`

## Verification Command
`cargo test -p amp-adapter-remote`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
