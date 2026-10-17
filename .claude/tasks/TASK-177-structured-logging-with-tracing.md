# TASK-177: Structured Logging with `tracing` to Rolling Log Files

## Plan Reference
- Plan: [PLAN-048](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-048-observability-and-diagnostics.md)
- Role: `qa_and_hardening`
- Priority: **P1**
- Audit findings: `QA-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-147

## Objectives
1. `tracing` + `tracing-subscriber` + `tracing-appender` in the composition root; replace every `eprintln!`; `RUST_LOG`/settings level; secrets redacted.

## Acceptance Criteria (user-visible / measurable)
- `grep -rn eprintln crates/*/src` returns nothing outside tests.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/logging.rs`
- `crates/*/src/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
