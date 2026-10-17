# TASK-185: Fuzz All Untrusted-Input Parsers & Add Size Limits

## Plan Reference
- Plan: [PLAN-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-050-release-grade-quality-assurance.md)
- Role: `security`
- Priority: **P2**
- Audit findings: `QA-07`, `SEC-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. `cargo-fuzz` targets: M3U/PLS/XSPF, `.wsz` loader (with entry/total size caps), ICY parser, Soulseek codec, remote JSON.
2. Nightly 10-minute fuzz job; crashes become regression tests.

## Acceptance Criteria (user-visible / measurable)
- Zip-bomb `.wsz` rejected with bounded memory; no fuzz crashes after 1 h each.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `fuzz/`
- `crates/amp-core-engine/src/skin.rs`

## Verification Command
`cargo +nightly fuzz run wsz -- -max_total_time=60`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
