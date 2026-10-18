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
- `deny.toml`: license allow-list (MPL-2.0-compatible), advisories, wildcard/source bans and the native-code deny list. `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok` locally. Added as a CI job; Dependabot configured for cargo (GUI stack grouped) and GitHub Actions.
- Workspace crates marked `publish = false`.
- Two advisories are ignored **with owners**: RUSTSEC-2025-0141 (bincode unmaintained → TASK-148) and RUSTSEC-2025-0003 (out-of-bounds read in `fast-float` via `rusty_ytdl` → `boa_engine`, which parses remote YouTube JavaScript; no upstream fix → TASK-157).
- Open: consolidating shared versions into `[workspace.dependencies]` and removing the `block 0.1.6` future-incompat warning (needs the eframe upgrade).

## Status
- **Status:** Pending
