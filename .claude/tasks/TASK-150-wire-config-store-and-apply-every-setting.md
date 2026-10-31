# TASK-150: Wire ConfigStore: Persist Settings & Apply (or Remove) Every Option

## Plan Reference
- Plan: [PLAN-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-041-persistent-library-config-and-secrets.md)
- Role: `storage_and_state`
- Priority: **P1**
- Audit findings: `DAT-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-147, TASK-125

## Objectives
1. Load `ConfigStore` at startup in the composition root; GUI/TUI settings edit through it; hot-reload via `ConfigChanged`.
2. Audit every `AppConfig` field: give it a consumer or remove it from schema/UI (e.g. provider toggles for nonexistent providers).

## Acceptance Criteria (user-visible / measurable)
- Changing each setting, restarting, shows the setting kept and effective (checklist in task).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/config_store.rs`
- `crates/amp-adapter-gui/src/settings_panel.rs`
- `crates/amp-domain/src/config.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
- Wired `ConfigStore` into `MultiplayGuiApp` (`save_and_apply_config`, `apply_config_to_engine`) and verified settings persistence and live engine application across restarts via `cargo test -p amp-adapter-gui config_survives_restart_and_applies_to_engine`.

## Status
- **Status:** Pending
