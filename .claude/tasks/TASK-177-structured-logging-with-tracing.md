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

## Implementation Evidence (2026-10-31, branch feat/plan-048-observability)
- `amp_core_engine::diagnostics::{init_logging, file_subscriber}`: tracing + tracing-subscriber (env-filter) + tracing-appender, daily `multiplay.YYYY-MM-DD.log`, 7 kept, `RUST_LOG` overrides the `info` default. Both composition roots (GUI `main.rs`, TUI `amp-app/src/main.rs`) install it; log dir `~/.multiplay/logs` (`AppPaths::log_dir`).
- Engine events are logged where they are emitted (`EventBus::emit_event`: playback errors warn, track starts info); provider search failures/timeouts and remote-control start/stop/errors are logged.
- `eprintln!` replaced in cpal actor and decoder; the only remaining one reports that the log file itself could not be opened.
- Secrets: `redact_config` (password/secret/token/credential/api_key/cookie keys).
- Tests: `crates/amp-core-engine/tests/diagnostics.rs` (file written, level filtering, redaction). Real run: the TUI in a pty wrote `multiplay-tui 0.1.0 (b5a812d) starting os="macos"` to the log.
