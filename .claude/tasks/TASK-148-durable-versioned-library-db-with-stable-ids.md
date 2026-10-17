# TASK-148: Durable, Versioned Library DB with Stable Track IDs

## Plan Reference
- Plan: [PLAN-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-041-persistent-library-config-and-secrets.md)
- Role: `storage_and_state`
- Priority: **P0**
- Audit findings: `DAT-02`, `DAT-03`, `DAT-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-147

## Objectives
1. Single persistent `library.redb` in the data dir for both front-ends (remove temp-dir DB).
2. Deterministic `TrackId` = hash of canonical source locator; upsert on rescan.
3. Versioned record envelope + migrations; tolerate and quarantine corrupt records.
4. Indexes: by artist, album, path, source kind, date added; session playlist stores TrackIds.

## Acceptance Criteria (user-visible / measurable)
- Rescan of an unchanged 10k-file library adds 0 records; adding a field to `TrackMetadata` keeps old DBs readable (migration test).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-storage/src/lib.rs`
- `crates/amp-domain/src/lib.rs`

## Verification Command
`cargo test -p amp-adapter-storage`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
