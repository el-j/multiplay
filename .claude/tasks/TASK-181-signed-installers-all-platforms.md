# TASK-181: Signed Installers: macOS DMG (notarized), Windows MSI, Linux AppImage/.deb/Flatpak

## Plan Reference
- Plan: [PLAN-049](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-049-release-engineering-and-distribution.md)
- Role: `devops`
- Priority: **P1**
- Audit findings: `BLD-04`, `BLD-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-123, TASK-180

## Objectives
1. macOS: `.app` with Info.plist/icon, codesign + notarize (`rcodesign`), DMG.
2. Windows: `cargo-wix` MSI, Authenticode signing, Start menu entry.
3. Linux: AppImage, `.deb`, Flatpak manifest with desktop file, icon, MIME types.

## Acceptance Criteria (user-visible / measurable)
- Fresh VMs install and launch without security warnings (macOS/Windows) and appear in app launchers.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `.github/workflows/release.yml`
- `packaging/`

## Verification Command
`Release dry-run on tag in a fork`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
