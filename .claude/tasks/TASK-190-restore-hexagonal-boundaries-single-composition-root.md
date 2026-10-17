# TASK-190: Restore Hexagonal Boundaries & Single Composition Root

## Plan Reference
- Plan: [PLAN-051](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-051-codebase-consolidation-and-architecture-integrity.md)
- Role: `architecture`
- Priority: **P2**
- Audit findings: `ARC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-126

## Objectives
1. Move `PlayerCommand`/`PlayerEvent`/`EventBus` contracts to `amp-ports`; `amp-adapter-remote` depends only on ports.
2. `amp-adapter-gui` depends on ports + egui only; all adapter construction in `amp-app`.
3. Add a `scripts/check_architecture.sh` (cargo metadata) enforcing the allowed dependency graph in CI.

## Acceptance Criteria (user-visible / measurable)
- Architecture check passes in CI.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-ports/`
- `crates/amp-adapter-remote/Cargo.toml`
- `crates/amp-adapter-gui/Cargo.toml`
- `scripts/check_architecture.sh`

## Verification Command
`./scripts/check_architecture.sh`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
