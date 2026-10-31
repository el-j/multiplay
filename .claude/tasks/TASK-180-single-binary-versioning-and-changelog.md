# TASK-180: Single `multiplay` Binary, Versioning & CHANGELOG

## Plan Reference
- Plan: [PLAN-049](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-049-release-engineering-and-distribution.md)
- Role: `devops`
- Priority: **P2**
- Audit findings: `BLD-07`, `ARC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-126

## Objectives
1. One `multiplay` binary in `amp-app` (`--tui`, `--headless --remote`), GUI default; move GUI main out of the adapter crate.
2. SemVer from workspace version; `--version` shows git hash; CHANGELOG via conventional commits (git-cliff / release-plz).

## Acceptance Criteria (user-visible / measurable)
- `multiplay --version` and `multiplay --tui` work from release artifacts.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/main.rs`
- `CHANGELOG.md`
- `cliff.toml`

## Verification Command
`cargo run -p amp-app -- --version`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31, branch feat/beta-release-polish) — partial
- Done: `--version`/`-V` on `multiplay-gui` and `amp-app` (`multiplay 0.1.0 (abc1234)`); `build-support/build_info.rs` takes the release tag from `MULTIPLAY_VERSION` (set in release.yml) and the commit from `GITHUB_SHA` or git. Tests `crates/amp-app/tests/version.rs`, `crates/amp-adapter-gui/tests/about.rs`.
- Done: hand-written `CHANGELOG.md` (Keep a Changelog), shipped in every download.
- Open: single `multiplay` binary with `--tui`/`--headless` (objective 1); git-cliff automation.
