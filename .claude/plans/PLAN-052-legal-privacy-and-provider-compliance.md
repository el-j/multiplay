# PLAN-052: Legal, Privacy & Provider Compliance

## Plan Overview
- **ID:** `PLAN-052`
- **Title:** Legal, Privacy & Provider Compliance
- **Role:** `product, security`
- **Milestone:** M5 — Ship (start in M0)
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-034`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Ship responsibly: opt-in gray-area providers with disclosures, a privacy statement, and trademark-safe branding.

## Objectives
1. **TASK-192** (P1): Provider ToS Review, Opt-In Disclosures & Download Policy
2. **TASK-193** (P2): Privacy Statement & Network Activity Inventory
3. **TASK-194** (P1): Trademark-Safe Branding (Remove 'WINAMP' from UI Titles)

## Tasks
- [ ] [TASK-192: Provider ToS Review, Opt-In Disclosures & Download Policy](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-192-provider-tos-review-and-opt-in-disclosures.md)
- [ ] [TASK-193: Privacy Statement & Network Activity Inventory](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-193-privacy-statement-and-network-inventory.md)
- [ ] [TASK-194: Trademark-Safe Branding (Remove 'WINAMP' from UI Titles)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-194-trademark-safe-branding.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
