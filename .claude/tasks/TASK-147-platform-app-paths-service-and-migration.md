# TASK-147: Platform AppPaths Service (config/data/cache/log) & Migration

## Plan Reference
- Plan: [PLAN-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-041-persistent-library-config-and-secrets.md)
- Role: `storage_and_state`
- Priority: **P1**
- Audit findings: `DAT-01` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. `AppPaths` using the `directories` crate; injected into every component; `MULTIPLAY_HOME` override for portable mode/tests.
2. One-time migration from `~/.multiplay/*` and `~/.config/multiplay/*`; expand `~` in config paths.

## Acceptance Criteria (user-visible / measurable)
- Works on Windows without `HOME`; migration test with fixture dirs.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-storage/src/paths.rs`
- `crates/amp-app/src/`

## Verification Command
`cargo test -p amp-adapter-storage paths`

## Manual Verification
- Verified `AppPaths` (`from_env`, `from_env_map`, `from_portable_root`, `migrate_from_legacy_home`, `expand_tilde`) working on Windows without `HOME`, honoring `MULTIPLAY_HOME`, and migrating `~/.multiplay/*` and `~/.config/multiplay/*` via `cargo test -p amp-adapter-storage paths` (2 passed).

## Status
- **Status:** Pending
