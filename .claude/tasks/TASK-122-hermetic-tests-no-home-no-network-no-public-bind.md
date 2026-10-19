# TASK-122: Hermetic Tests: No $HOME Writes, No Network, No 0.0.0.0 Binds

## Plan Reference
- Plan: [PLAN-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-036-green-ci-and-hermetic-tests.md)
- Role: `qa_and_hardening`
- Priority: **P1**
- Audit findings: `QA-02`, `DAT-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-147

## Objectives
1. Inject `AppPaths` into GUI/TUI state; tests use `tempfile` dirs.
2. Remove `std::env::set_var` from tests (inject a `DialogBackend` trait instead).
3. Network adapters take an injected base URL/client; tests use a local `tokio` test server or recorded fixtures.
4. Add a CI guard that fails if `~/.multiplay` or `~/.config/multiplay` is modified during `cargo test` (HOME pointed to a sandbox).

## Acceptance Criteria (user-visible / measurable)
- `HOME=$(mktemp -d) cargo test --workspace` leaves no files outside the temp dirs; tests pass offline.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/app.rs`
- `crates/amp-adapter-gui/src/file_dialog.rs`
- `.github/workflows/ci.yml`

## Verification Command
`HOME=$(mktemp -d) cargo test --workspace --offline`

## Manual Verification
- `AppPaths` injected into the GUI (session, layout, remote sessions, music dir); tests use per-test temp roots with music scanning disabled.
- Cancelled file/folder dialogs no longer trigger a `~/Music` rescan (UX-07).
- The test that mutated the process-global `CI` env var (and removed it, affecting parallel tests) was replaced; `cfg!(test)` already disables native dialogs.
- `scripts/check_hermetic_tests.sh` / `make test-hermetic` runs the suite with a sandboxed HOME and fails if any multiplay state is written; CI uses it on all 3 OSes. Local run: PASS.
- Open: network adapters still build real HTTP clients (objective 3, injected base URLs / local test servers).

- 2026-10-19: CI green on ubuntu/macos/windows (run https://github.com/el-j/multiplay/actions/runs/39215214130, commit after 3 CI-found fixes: winit X11/Wayland features, stream-underrun data loss, TUI lock starvation; hermetic guard tolerates Windows INetCache churn). Tests job 71-341 s, previously 70 min on Linux.

## Status
- **Status:** Pending
