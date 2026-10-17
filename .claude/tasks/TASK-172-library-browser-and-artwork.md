# TASK-172: Library Browser (Artists/Albums/Tracks) & Artwork

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-148

## Objectives
1. Views: Artists, Albums (grid with covers), Tracks (sortable), Recently Added, Sources.
2. Embedded art extraction (lofty) + remote cover URIs, cached in cache dir with size limits.
3. Virtualized lists for 100k tracks.

## Acceptance Criteria (user-visible / measurable)
- Scrolling 100k tracks stays at 60 FPS; album grid shows covers.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/library_window.rs`
- `crates/amp-adapter-storage/src/artwork.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
