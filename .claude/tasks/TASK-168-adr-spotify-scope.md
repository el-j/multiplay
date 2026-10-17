# TASK-168: ADR: Spotify — Drop Playback, Offer Web-API Playlist Import & Cross-Source Matching

## Plan Reference
- Plan: [PLAN-046](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-046-p2p-mesh-and-spotify-scope-decisions.md)
- Role: `architecture`
- Priority: **P1**
- Audit findings: `SRC-02`, `SRC-18`, `LEG-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. ADR weighing librespot (Premium-only, ToS violation, breakage) vs. official Web API metadata import + matching to library/Bandcamp/YouTube/Soulseek.
2. Remove fake `PureSpotifyAdapter` from production; fix `spotify:track:` double-prefix export bug either way.

## Acceptance Criteria (user-visible / measurable)
- ADR merged; no Spotify playback UI unless a real adapter ships.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `docs/adr/ADR-003-spotify.md`
- `crates/amp-adapter-spotify/`

## Verification Command
`test -f docs/adr/ADR-003-spotify.md`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
