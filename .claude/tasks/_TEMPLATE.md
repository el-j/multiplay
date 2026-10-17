# TASK-NNN: Task Title

## Plan Reference
- Plan: [PLAN-NNN](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-NNN-name.md)
- Role: `<role>`
- Priority: **P0 / P1 / P2**
- Audit findings: `<AUDIT-ID>` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Concise bullet points describing concrete implementation steps.
2. Every item must trace back to verifiable code and automated tests.

## Acceptance Criteria (user-visible / measurable)
- Measurable criterion 1 (e.g., capability verified on release binary).
- Measurable criterion 2 (e.g., zero memory allocation on CPAL audio thread).
- **Mocks Rule:** In-memory mocks may be used for unit testing isolated components, but NEVER mark a provider, network, or storage task Completed. Real adapters or recorded golden fixtures are required.
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length constraint enforced ($\le 300$ lines per file).

## Key Files
- `crates/<crate>/src/<file>.rs`

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p <crate>`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run -p amp-app -- ...`
- **Observed Behavior:** Step-by-step description of release/dev run and verified output.
- **Evidence Links:** Screenshot, terminal session log, or CI run URL.

## Status
- **Status:** Pending
