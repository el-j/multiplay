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
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
