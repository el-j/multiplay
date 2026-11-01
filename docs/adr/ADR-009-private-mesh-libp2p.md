# ADR-009: The private mesh on libp2p with a pre-shared key

- **Status:** Accepted
- **Date:** 2026-11-01
- **Tasks:** `TASK-167` (`PLAN-046`)

## Context

`amp-adapter-p2p` was a placeholder (`SecurePeerNode`): an in-memory catalog with no network.
The product owner chose libp2p for the real mesh. The mesh is for one's own devices and friends,
not strangers, so membership must be closed, and the pure-Rust rules still apply.

## Decision

- **libp2p 0.56** (0.57's `libp2p-swarm` pins a `wasm-bindgen-futures` that conflicts with
  eframe). Transport: TCP, then **pnet** with the mesh code as the 32-byte pre-shared key, then
  Noise and Yamux. A device without the code fails the pnet handshake; there is no separate
  invitation or trust list.
- **Mesh codes** are the key in Crockford base32 (52 characters, groups of four), forgiving of
  case, spaces and look-alike letters. mDNS announces only a hash of the code (`discovery_id`).
- **Request-response** (`/multiplay/mesh/1`, CBOR): `Search { query }` → up to 50 shared tracks;
  `Fetch { id, offset, len }` → 256 KiB chunks of a shared file. Files are addressed by an id
  derived from path and size, so a peer can only fetch what is shared. Playback streams the file
  into a `SharedPrecacheBuffer` while the first chunk is already decodable.
- **Gossipsub** carries jam-session operations (the RGA CRDT in `amp_domain::jam_crdt`, now with
  buffered out-of-order operations and per-voter votes) on topic `multiplay-jam-<discovery_id>`.
- **Discovery:** `mdns-sd` instead of `libp2p-mdns`, whose `hickory-proto` 0.25 has open
  advisories (RUSTSEC-2026-0118/0119). Platform bindings pulled in by libp2p (`netlink-sys`,
  `system-configuration-sys`) are listed in `scripts/pure_rust_allowlist.txt`.
- **Off by default**; the desktop app runs the node on its own small runtime. The terminal app
  registers an empty provider for now.

## Consequences

- Members on other networks cannot find each other yet (no relay, rendezvous or hole punching);
  they can still be dialled by address in code. Adding a relay is a later decision because it
  needs a server.
- Jam sessions have transport and a convergent CRDT, but no window yet.
- The old `SecurePeerNode` and its "trusted group" are gone.
