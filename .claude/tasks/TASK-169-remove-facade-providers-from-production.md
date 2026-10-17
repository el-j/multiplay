# TASK-169: Remove/Feature-Gate Facade Providers from Production Builds

## Plan Reference
- Plan: [PLAN-046](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-046-p2p-mesh-and-spotify-scope-decisions.md)
- Role: `integrations`
- Priority: **P0**
- Audit findings: `SRC-01` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Move in-memory Soulseek/P2P/Spotify implementations into `amp-test-support` fakes.
2. Production composition registers only real providers; UI pills/toggles shown only for compiled-in, working providers.

## Acceptance Criteria (user-visible / measurable)
- Release build shows no SOULSEEK/P2P/SPOTIFY/BANDCAMP/SOUNDCLOUD controls until their real adapters land.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/`
- `crates/amp-adapter-gui/src/playlist_window.rs`
- `crates/amp-adapter-gui/src/settings_panel.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
