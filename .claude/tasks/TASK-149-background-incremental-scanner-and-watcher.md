# TASK-149: Background Incremental Scanner, Symlink-Safe, with File Watcher

## Plan Reference
- Plan: [PLAN-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-041-persistent-library-config-and-secrets.md)
- Role: `storage_and_state`
- Priority: **P1**
- Audit findings: `DAT-04`, `SEC-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-148

## Objectives
1. Async scanner task: no symlink following by default (visited inode set when enabled), mtime/size incremental, cancellable, progress events.
2. `notify`-based watcher honoring `watch_filesystem`; one canonical supported-extension list.
3. Remove synchronous scans from GUI startup and dialog-cancel paths.

## Acceptance Criteria (user-visible / measurable)
- GUI window appears < 1 s with a 50k-file library; symlink loop fixture terminates.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-storage/src/scanner.rs`
- `crates/amp-adapter-gui/src/app_state.rs`

## Verification Command
`cargo test -p amp-adapter-storage scanner`

## Manual Verification
- Verified symlink-safe traversal (`ScanOptions`), incremental `(mtime_secs, size_bytes)` fingerprinting (`IncrementalScanCache`), and cancellation via `cargo test -p amp-adapter-storage scanner` (`scanner_terminates_on_symlink_loop_fixture`, `scanner_incremental_skips_unchanged_and_honors_cancellation`, 5 passed).

## Status
- **Status:** Pending
