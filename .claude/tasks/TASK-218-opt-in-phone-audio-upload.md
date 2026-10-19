# TASK-218: Opt-In Phone Audio Upload (Off by Default)

## Plan Reference
- Plan: [PLAN-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-056-remote-and-sharing-window-host-moderated-requests.md)
- Design: [Signature features design §7](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `security`
- Priority: **P2**
- Audit findings: —
- Depends on: TASK-215, TASK-185

## Objectives
1. Host enables uploads per session; otherwise the upload endpoint answers 403.
2. Size (e.g. 50 MB) and format limits; decode-probe with the fuzzed decoder before accepting; stored in the cache dir, cleaned on exit.
3. Uploaded tracks enter the request queue like any other request.

## Acceptance Criteria (user-visible / measurable)
- Security tests: disabled → 403; oversize, non-audio and malformed files rejected; no path traversal in stored names.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-remote/src/upload.rs (new)`
- `crates/amp-adapter-remote/tests/security.rs`

## Verification Command
`cargo test -p amp-adapter-remote upload`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
