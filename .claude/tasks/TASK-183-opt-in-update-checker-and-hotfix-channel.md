# TASK-183: Opt-In Update Checker & Provider Hotfix Channel

## Plan Reference
- Plan: [PLAN-049](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-049-release-engineering-and-distribution.md)
- Role: `devops`
- Priority: **P3**
- Audit findings: `SRC-09` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-181

## Objectives
1. Opt-in check against GitHub Releases API (no telemetry); notify about provider-breakage hotfix releases.

## Acceptance Criteria (user-visible / measurable)
- Disabled by default; one HTTPS request per day max when enabled.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/update_check.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-049-release)
- Decision logic (`amp_core_engine::update_check`): newest newer release by SemVer (betas only when chosen; 0.1.0 beats its betas), `check_is_due` once per 24 h (clock set back does not block), `ReleaseInfo::fixes_online_source` from "Provider fix" / "[hotfix]" notes. Tests `amp-core-engine/tests/update_check.rs`.
- Fetch (`amp_adapter_http::releases`): one GET to `https://api.github.com/repos/el-j/multiplay/releases?per_page=20` with the honest User-Agent; drafts skipped; garbage/rate-limit answers are errors. Tests with a GitHub fixture + an ignored live test (passed: 0 published releases).
- App: `config.updates` (off by default, betas on, last check remembered before asking); background thread; "NEW VERSION (fixes online sources)" label on the faceplate opens the release page; Preferences → About switches with what is sent. Tests `amp-adapter-gui/tests/update_check.rs` (off → no request; on → once a day + offer; failure quiet).
- docs/privacy.md lists the request.
