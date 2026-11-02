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

## Implementation Evidence (2026-11-01, branch feat/plan-049-release)
- Windows MSI: `packaging/windows/multiplay.wxs` (WiX 5 via `dotnet tool`), Program Files + Start menu, `AllowSameVersionUpgrades` with version `<cargo>.<run>`; signed with Azure Artifact Signing when configured.
- Debian: cargo-deb metadata in `crates/amp-adapter-gui/Cargo.toml` (both apps, desktop entry, icons 16–512, docs), version `0.1.0~beta.N`; built for x86_64 and aarch64.
- Release dry run https://github.com/el-j/multiplay/actions/runs/41323507030: all jobs green; artifacts: dmg, msi, zip, 2× deb, 2× AppImage, 2× tar.gz, SHA256SUMS (all verify). The DMG's apps report `multiplay 0.1.0-dev.c16f06e (c16f06e)`; the license bundle is in the app's Resources.
- Fixed after the dry run: WiX's `.wixpdb` is removed from dist.
- Not done: notarization needs the Apple secrets; Flatpak manifest (needs generated cargo sources); installing the MSI/.deb on fresh VMs by hand.
