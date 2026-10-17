# TASK-120: Fix rustfmt Drift & Install Linux System Dependencies in CI

## Plan Reference
- Plan: [PLAN-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-036-green-ci-and-hermetic-tests.md)
- Role: `devops`
- Priority: **P0**
- Audit findings: `BLD-02` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-113

## Objectives
1. Run `cargo fmt --all` and commit.
2. Install `libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev libx11-dev libxi-dev libgl1-mesa-dev` on Ubuntu jobs.
3. Add `--locked` to all cargo invocations; set `RUSTDOCFLAGS=-D warnings` on the doc job.

## Acceptance Criteria (user-visible / measurable)
- `main` CI is green on ubuntu, macos and windows.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `.github/workflows/ci.yml`

## Verification Command
`cargo fmt --all -- --check && cargo clippy --workspace --all-targets --locked -- -D warnings`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
