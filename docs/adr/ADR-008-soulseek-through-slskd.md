# ADR-008: Soulseek through slskd, not our own protocol client

- **Status:** Accepted
- **Date:** 2026-11-01
- **Tasks:** `TASK-162`–`TASK-166` (`PLAN-045`: Real Soulseek Client)

## Context

PLAN-045 planned a full Soulseek client in Rust: server session, search, peer connections
(direct and firewalled), transfers and sharing. That is a large protocol implementation to
maintain, and sharing files with strangers is a product and legal decision of its own. The
product owner decided: skip our own implementation, offer "Use Soulseek" for people who already
run a Soulseek client, and install one when the user switches it on.

## Decision

- multiplay talks to **slskd** (AGPL-3.0, .NET, REST API at `/api/v0` with `X-API-Key`), the
  one Soulseek client with a documented API. Desktop clients such as Nicotine+ and SoulseekQt
  have none.
- **Search:** `POST /searches`, then `GET /searches/{id}?includeResponses=true` until complete;
  playable, unlocked audio becomes results, best peers (free slot, short queue, speed) first.
- **Play:** a download batch with an explicit destination (`multiplay/<id>`), polled through
  `GET /transfers/downloads/{user}` until `Succeeded`, then the local file is played. Soulseek
  can take minutes, so resolvers may now ask the engine for a longer resolve timeout
  (`StreamResolverPort::resolve_timeout`).
- **Two modes:** connect to the user's slskd (address + API key), or **install on request**: the
  official release zip for this system from GitHub (version pinned, SHA-256 from GitHub's asset
  digests pinned in code), unpacked with zip-slip checks into `~/.multiplay/slskd/`, run while
  multiplay runs on `127.0.0.1` with a generated API key and **no shares**. Settings go through
  its environment, so the password is never on a command line.
- **Licensing:** slskd is not bundled, linked or modified; it is downloaded from upstream at the
  user's request and runs as a separate program, so its AGPL does not extend to multiplay.
- The old placeholder (`SoulseekAdapter` with in-memory "transfers") is removed.

## Consequences

- No Soulseek protocol code to keep up with; slskd tracks protocol changes.
- slskd is ~55 MB and needs nothing else (self-contained .NET).
- Verified against a fake slskd that answers the routes from slskd's source; a real slskd and
  account have not been exercised in CI.
- Sharing (uploads) is off; people who want to share configure their own slskd.
