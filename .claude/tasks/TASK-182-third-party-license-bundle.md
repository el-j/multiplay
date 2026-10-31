# TASK-182: Third-Party License Bundle & About Dialog

## Plan Reference
- Plan: [PLAN-049](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-049-release-engineering-and-distribution.md)
- Role: `devops`
- Priority: **P1**
- Audit findings: `LEG-02` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-124

## Objectives
1. `cargo-about` generates `THIRD_PARTY_LICENSES.html` in release jobs; included in installers; About dialog links it.

## Acceptance Criteria (user-visible / measurable)
- Every artifact contains the license bundle.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `about.toml`
- `about.hbs`
- `.github/workflows/release.yml`

## Verification Command
`cargo about generate about.hbs > /dev/null`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31, branch feat/beta-release-polish)
- `about.toml` (accepted list = deny.toml allow list, own crates ignored) + `about.hbs` (responsive, light/dark HTML).
- CI `supply-chain` job installs cargo-about 0.9.3, generates `THIRD_PARTY_LICENSES.html` and uploads it; release.yml copies it into the dmg (also `Multiplay.app/Contents/Resources`), the Linux tarballs and AppImage, and the Windows zip.
- Preferences → About opens the bundled file (`amp-adapter-gui/src/about.rs`, `licenses_file_near`); test `crates/amp-adapter-gui/tests/about.rs`.
- Local: `cargo about generate --offline about.hbs -o …` → 575 KB, 284 license texts, no amp-* crates.
- Not yet verified: a tagged release run (needs a tag).
