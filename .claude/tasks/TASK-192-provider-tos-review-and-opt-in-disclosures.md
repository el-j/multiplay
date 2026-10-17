# TASK-192: Provider ToS Review, Opt-In Disclosures & Download Policy

## Plan Reference
- Plan: [PLAN-052](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-052-legal-privacy-and-provider-compliance.md)
- Role: `product`
- Priority: **P1**
- Audit findings: `LEG-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-168

## Objectives
1. Legal review memo per provider; each gray-area provider opt-in with an in-app disclosure; downloads off by default; Bandcamp results link to purchase pages.
2. Providers are Cargo features so distributions can omit them.

## Acceptance Criteria (user-visible / measurable)
- Default build/config contacts no third-party service until the user opts in.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `docs/legal/providers.md`
- `crates/amp-app/Cargo.toml`

## Verification Command
`cargo build -p amp-app --no-default-features`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
