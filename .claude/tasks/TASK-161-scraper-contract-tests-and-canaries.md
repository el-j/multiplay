# TASK-161: Recorded-Fixture Contract Tests & Live Canaries for Scraping Adapters

## Plan Reference
- Plan: [PLAN-044](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-044-bandcamp-and-soundcloud-resolvers.md)
- Role: `qa_and_hardening`
- Priority: **P2**
- Audit findings: `QA-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-159, TASK-160

## Objectives
1. Shared provider contract test kit (search shape, resolve → decodable first 5 s).
2. Fixture refresh script; extend canary workflow to Bandcamp and SoundCloud.

## Acceptance Criteria (user-visible / measurable)
- Canaries run daily and open an issue on failure.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-test-support/src/provider_contract.rs`
- `.github/workflows/canary.yml`

## Verification Command
`cargo test --workspace provider_contract`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
