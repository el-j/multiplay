# PLAN-049: Release Engineering & Distribution

## Plan Overview
- **ID:** `PLAN-049`
- **Title:** Release Engineering & Distribution
- **Role:** `devops`
- **Milestone:** M5 — Ship
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-036`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Signed, installable, versioned artifacts for macOS, Windows and Linux, with license compliance and an update path.

## Objectives
1. **TASK-180** (P2): Single `multiplay` Binary, Versioning & CHANGELOG
2. **TASK-181** (P1): Signed Installers: macOS DMG (notarized), Windows MSI, Linux AppImage/.deb/Flatpak
3. **TASK-182** (P1): Third-Party License Bundle & About Dialog
4. **TASK-183** (P3): Opt-In Update Checker & Provider Hotfix Channel

## Tasks
- [ ] [TASK-180: Single `multiplay` Binary, Versioning & CHANGELOG](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-180-single-binary-versioning-and-changelog.md)
- [ ] [TASK-181: Signed Installers: macOS DMG (notarized), Windows MSI, Linux AppImage/.deb/Flatpak](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-181-signed-installers-all-platforms.md)
- [ ] [TASK-182: Third-Party License Bundle & About Dialog](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-182-third-party-license-bundle.md)
- [ ] [TASK-183: Opt-In Update Checker & Provider Hotfix Channel](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-183-opt-in-update-checker-and-hotfix-channel.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
