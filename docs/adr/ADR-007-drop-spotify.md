# ADR-007: Drop Spotify for now

- **Status:** Accepted
- **Date:** 2026-11-01
- **Task:** `TASK-168` (`PLAN-046`: P2P Mesh & Spotify Scope Decisions)

## Context

The `amp-adapter-spotify` crate was a placeholder: it checked a login and returned generated
audio (orchestration audit §3.1). Real playback would need `librespot`, which reverse-engineers
Spotify's protocol against its terms of service, requires a Premium account, and cannot be made
bit-perfect. The product owner decided to drop Spotify for now.

## Decision

- The crate is removed from the workspace, with its settings switch, its search filter and the
  `enable_spotify` setting.
- `AudioSource::Spotify` **stays in the domain model**, so playlists, sessions and libraries
  saved by older versions still load. Such entries do not play; the engine says
  "Spotify isn't supported: this playlist entry can't be played".
- Spotify links pasted into the search box are recognised (so they are not searched for as
  words) and refused with that reason (`amp_domain::source_url::unsupported_link`).
- Old `config.toml` files with `enable_spotify` still load; the key is ignored.

## Consequences

- One less placeholder that looked like a feature; no Spotify credentials are stored anywhere.
- Coming back later means a new ADR. The options then: Spotify's Web API for playlist import
  (matching tracks to other sources), not playback.
