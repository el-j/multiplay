# TASK-157: YouTube Format Selection, URL Refresh, Retry & Daily Canary

## Plan Reference
- Plan: [PLAN-043](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-043-youtube-resolver-and-downloader-hardening.md)
- Role: `integrations`
- Priority: **P0**
- Audit findings: `SRC-09` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-139

## Objectives
1. Select a decodable format (itag 140 AAC/MP4 until Opus/WebM supported; then prefer 251).
2. Refresh expired/403 URLs; retry with backoff; detect extractor breakage and surface `ProviderError::Unavailable`.
3. Scheduled CI canary resolving + decoding 5 s of a known CC-licensed video; alert on failure; document hotfix procedure for `rusty_ytdl` bumps.

## Acceptance Criteria (user-visible / measurable)
- 20 random popular videos play end-to-end (manual); canary green for 7 days.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-youtube/src/lib.rs`
- `.github/workflows/canary.yml`

## Verification Command
`cargo test -p amp-adapter-youtube && gh workflow run canary`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
