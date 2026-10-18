# TASK-116: Remote Daemon Off by Default, Loopback Bind, Explicit LAN Opt-In

## Plan Reference
- Plan: [PLAN-035](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-035-remote-control-security-hardening.md)
- Role: `security`
- Priority: **P0**
- Audit findings: `SEC-01`, `SEC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Remove the unconditional `remote.start_daemon()` from `MultiplayGuiApp::new()`.
2. Add `remote.enabled` (default false) and `remote.allow_lan` (default false) to `AppConfig`; bind `127.0.0.1` unless `allow_lan`.
3. Show a persistent "Remote ON" indicator in the main window while listening; settings toggle starts/stops the daemon.
4. Replace the `8.8.8.8` UDP trick with interface enumeration (`if-addrs`).

## Acceptance Criteria (user-visible / measurable)
- Fresh install: no listening socket (`lsof -i :9002` empty).
- Enabling LAN mode shows the macOS firewall prompt only after explicit user action.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/app_state.rs`
- `crates/amp-adapter-gui/src/remote_manager.rs`
- `crates/amp-domain/src/config.rs`

## Verification Command
`cargo test -p amp-adapter-gui remote && ./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
- `MultiplayGuiApp` no longer starts the daemon; `AppConfig.remote { enabled: false, allow_lan: false, port: 9002 }` (`#[serde(default)]` so old configs load).
- Settings → Remote: "Enable remote control" and "Allow devices on my network" toggles start/stop/rebind the daemon; busy port is reported in the UI. Loopback (`127.0.0.1`) unless LAN is allowed.
- Main window shows `● REMOTE ON` / `● REMOTE ON (LAN)` while listening.
- LAN address picked via `if-addrs` interface enumeration (no `8.8.8.8` trick).
- Evidence: launched `target/debug/multiplay-gui`, waited 10 s, `lsof -nP -a -p <pid> -iTCP -sTCP:LISTEN` → empty; `lsof -iTCP:9002` → empty.
- Tests: `remote_manager::tests::{new_manager_does_not_listen, enabling_binds_loopback_and_disabling_releases_the_port, busy_port_is_reported}`, `config::tests::remote_control_is_off_and_loopback_by_default`.
- Note: the toggle is not persisted across launches until settings persistence lands (PLAN-041); it always starts off.

## Status
- **Status:** Pending
