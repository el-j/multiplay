# TASK-124: Supply Chain: cargo-deny, cargo-audit, Dependabot & Workspace Pinning

## Plan Reference
- Plan: [PLAN-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-036-green-ci-and-hermetic-tests.md)
- Role: `devops`
- Priority: **P2**
- Audit findings: `BLD-03`, `BLD-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-120

## Objectives
1. Add `deny.toml` (licenses allow-list, advisories, bans for duplicate majors, sources).
2. Add `cargo deny check` and `cargo audit` jobs; Dependabot for cargo + actions.
3. Move all shared dependency versions into `[workspace.dependencies]`; resolve the `block 0.1.6` future-incompat warning (update eframe stack).

## Acceptance Criteria (user-visible / measurable)
- `cargo deny check` passes in CI; no future-incompat warnings.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `deny.toml`
- `.github/dependabot.yml`
- `Cargo.toml`
- `crates/*/Cargo.toml`

## Verification Command
`cargo deny check && cargo audit`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
