# TASK-217: Phone Web Client: Search, Paste Link, Request Status

## Plan Reference
- Plan: [PLAN-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-056-remote-and-sharing-window-host-moderated-requests.md)
- Design: [Signature features design §7](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_ui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-215, TASK-153

## Objectives
1. Phone UI: search host library and providers (through the host), paste a link, request; see own requests with live status.
2. Mobile-friendly, works offline-first on LAN, strict CSP kept.

## Acceptance Criteria (user-visible / measurable)
- Manual: iOS Safari and Android Chrome pair, search, request and see 'playing'.
- HTTP tests assert CSP and no third-party resources.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-remote/src/web_client.rs`
- `crates/amp-adapter-remote/src/http.rs`

## Verification Command
`cargo test -p amp-adapter-remote web`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
