# TASK-123: Repair the Release Workflow (Missing Files, Silent Failures)

## Plan Reference
- Plan: [PLAN-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-036-green-ci-and-hermetic-tests.md)
- Role: `devops`
- Priority: **P0**
- Audit findings: `BLD-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-120

## Objectives
1. Remove the reference to the nonexistent `Multiplay.command` (or restore it under `scripts/`).
2. Drop `|| true` on binary copies; fail loudly.
3. Copy `scripts/Info.plist` into the `.app`; produce checksums (`SHA256SUMS`).

## Acceptance Criteria (user-visible / measurable)
- A `v0.1.0-rc.1` tag produces artifacts on all 3 OSes.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `.github/workflows/release.yml`
- `scripts/bundle_build.sh`

## Verification Command
`act -j build-release (or a dry-run tag on a fork)`

## Manual Verification
- Removed the nonexistent `Multiplay.command` and every `|| true`; `if-no-files-found: error` on upload.
- macOS: `scripts/Info.plist` copied into `Multiplay.app/Contents`, GUI is the bundle executable (`CFBundleExecutable = Multiplay`), TUI shipped as `bin/multiplay-tui`; both macOS targets build on `macos-latest` (macos-13 runners are retired).
- Linux/Windows: GUI as `multiplay`, TUI as `multiplay-tui`; README and LICENSE included; Linux build deps installed; `--locked`.
- `SHA256SUMS` generated; tags containing `-` (e.g. `v0.1.0-rc.1`) publish as pre-releases.
- Pending: push a `v0.1.0-rc.1` tag and confirm artifacts on all 3 OSes.

## Status
- **Status:** Pending
