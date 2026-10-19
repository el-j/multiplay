# TASK-194: Trademark-Safe Branding (Remove 'WINAMP' from UI Titles)

## Plan Reference
- Plan: [PLAN-052](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-052-legal-privacy-and-provider-compliance.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-10`, `LEG-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Replace all 'WINAMP' window titles/labels with multiplay branding; keep nominative 'supports classic Winamp® skins (.wsz)' wording only.
2. Ensure no bundled bitmaps derive from the original Winamp base skin; design an original default skin and app icon.

## Acceptance Criteria (user-visible / measurable)
- `grep -rni winamp crates/*/src` only matches skin-format code/comments.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/`
- `crates/amp-adapter-tui/src/`
- `assets/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.
- 2026-10-19: user-visible "Winamp" branding removed: TUI titles (MULTIPLAY ::, EQUALIZER, PLAYLIST), `DenseWinampConsole` renamed `DenseConsole`, default theme "multiplay Classic", Settings "Compact mode" and "Skins (classic Winamp(R) .wsz format)" (nominative). `grep -rni winamp crates/*/src` now matches only skin-format code (`WinampTheme`, `WinampSkin`, .wsz parsing), comments and that one nominative label.
- Open: objective 2 (original default skin and app icon).

## Status
- **Status:** Pending
