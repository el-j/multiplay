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
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
