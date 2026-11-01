# TASK-171: First-Run Onboarding Wizard

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-149, TASK-150

## Objectives
1. Steps: pick music folders (background scan w/ progress), choose output device, opt into providers with honest notes, remote control off by default.
2. Re-runnable from settings.

## Acceptance Criteria (user-visible / measurable)
- Fresh-profile usability test: first track playing < 60 s.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/onboarding.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-047-ux-2)
- `crates/amp-adapter-gui/src/onboarding.rs`: steps Music folders (~/Music suggested, add/remove; folder dialog off the UI thread) → Output device → Online sources (YouTube / Bandcamp with what each sends) → Remote control (off) → Start. Shown once in the Preferences overlay when `ui.first_run_done` is false (`#[serde(default)]`, so existing configs see it once); Skip also marks it done; Preferences → About → "Run the first-run setup again".
- Finishing scans the chosen folders on a background thread with a live count (`import_jobs.rs`, `import_paths_with_progress`), then adds them to the playlist and library ("Added N tracks").
- Online sources are now real switches: search, phone search and request previews only ask the providers switched on in Preferences → Accounts; with both off nothing is sent (privacy.md updated).
- Tests: `tests/onboarding.rs` (step order, choices land in settings, background scan with progress, skip), `tests/omnibox.rs` (switched-off provider not asked; all off → nothing sent).
- Not done: the "first track playing in under 60 s" usability check by hand.
