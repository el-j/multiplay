# AUDIT-05 — Sources, Search & Downloader

Part of [AUDIT-2026-10-17](README.md).

## Provider reality matrix

| Source | Search | Stream | Status in orchestrator | Reality |
| :--- | :---: | :---: | :--- | :--- |
| Local files | n/a (library search unwired) | ✅ | Completed | Works. Scanner issues in DAT-04. |
| Direct HTTP / radio | ❌ | ⚠️ | Completed | Plays simple files; radio corrupted by ICY (SRC-04); unbounded RAM (AUD-10). |
| YouTube (`rusty_ytdl 0.7.4`) | ✅ (API exists) | ⚠️ | Completed | Search code real but unwired; stream likely picks Opus/WebM, which can't be decoded (SRC-09). |
| Spotify | ❌ | ❌ | Completed | In-memory byte cache; any token "authenticates". |
| Soulseek | ❌ | ❌ | Completed | In-memory catalog; no socket code; some opcodes wrong. |
| P2P mesh | ❌ | ❌ | Completed | In-memory catalog; any non-empty token = trusted peer. |
| Bandcamp | ❌ | ❌ | (enabled in config UI) | Does not exist (enum variant + URL sniffing only). |
| SoundCloud | ❌ | ❌ | (enabled in config UI) | Does not exist. |
| SMB share | ❌ | ❌ | — | Enum variant only. |

## SRC-01 — Mock providers are wired into production builds · P0
**Evidence:** The TUI registers `SoulseekAdapter::default()` (username `multiplay_guest`, never logged in) and `SecurePeerNode::default()` (peer id `12D3KooWTestPeerMultiplayDefault`) as search providers (`crates/amp-app/src/app_state.rs`). The GUI shows SOULSEEK and P2P filter pills (`playlist_window.rs`) and Spotify/SoundCloud/Bandcamp toggles (`settings_panel.rs`), all non-functional.
**Fix:** Production builds must only show sources that work. Gate mock adapters behind `#[cfg(test)]` or a `dev-mocks` feature. → TASK-169

## SRC-02 — Spotify adapter is a facade · P1
**Evidence:** `crates/amp-adapter-spotify/src/lib.rs:18`, a `HashMap<String, Vec<u8>>`. No librespot, no OAuth, no search. `export_m3u8` writes `spotify:track:{track_uri}` where `track_uri` already contains `spotify:track:` (`playlist.rs:94`).
**Decision needed:** see SRC-18. → TASK-168

## SRC-03 — Soulseek & P2P adapters are facades · P1
See SRC-13 … SRC-17.

## SRC-04 — Internet radio: ICY metadata requested but never stripped · P0 for radio
**Evidence:** The `Icy-MetaData: 1` default header is set (`crates/amp-adapter-http/src/lib.rs:22`). The server then interleaves metadata blocks every `icy-metaint` bytes. The bytes are pushed verbatim into the decoder buffer, and nothing reads `icy-metaint`, so MP3/AAC frames are corrupted every ~16 KB: glitches, resyncs and decode errors.
**Fix:** Parse `icy-metaint`, demux the metadata blocks, and surface `StreamTitle` as now-playing. → TASK-137

## SRC-05 — Radio playlists (.pls/.m3u/.xspf URLs) are not resolved · P1
**Evidence:** The resolver's own test uses `https://somafm.com/groovesalad130.pls`, but a PLS body would be fed to Symphonia as audio.
**Fix:** Content-type and extension sniffing, then resolve the first reachable stream entry, with failover. → TASK-137

## SRC-06 — No HTTP range requests · P2
**Evidence:** A single `GET`. Seeking past downloaded bytes clamps silently, and `byte_len` is `None`, so Symphonia can't compute duration for CBR files.
**Fix:** `Accept-Ranges` detection, range-based reader, `Content-Length` → `byte_len`. → TASK-138

## SRC-07 — Search coordinator swallows provider errors · P2
**Evidence:** `p.query(&q).await.unwrap_or_default()` (`crates/amp-core-engine/src/filtered_search.rs:57`). `AuthenticationRequired`, `RateLimited` and `Network` all collapse into "no results". Results arrive only when *all* providers finish (no streaming), and there is no ranking or dedupe.
**Fix:** Return per-provider outcomes, stream results incrementally, and show a provider status chip in the UI. → TASK-154, TASK-155

## SRC-08 — YouTube identifiers are stored as full URLs · P1
**Evidence:** `ingest_url` stores the whole URL in `AudioSource::YouTube { video_id }` (`crates/amp-adapter-gui/src/app_state.rs:196`), and so does M3U import. Export then produces `https://www.youtube.com/watch?v=https://www.youtube.com/watch?v=…` (`playlist.rs:91`). Playlists, Shorts, `music.youtube.com` and `youtu.be/?t=` aren't handled.
**Fix:** A URL → ID normalizer in the domain layer, with unit tests for every URL form. → TASK-156

## SRC-09 — YouTube stream format likely undecodable · P0 (verify)
**Evidence:** `VideoQuality::HighestAudio` + `VideoSearchOptions::Audio` (`crates/amp-adapter-youtube/src/lib.rs`) normally selects itag 251 (Opus in WebM), the highest-bitrate audio format. The decoder enables neither an MKV/WebM demuxer nor an Opus codec (AUD-11). Expected result: "Unrecognized audio format" for most videos. There is also no handling for expiring signed URLs, throttled `n`-parameter downloads, or `rusty_ytdl` breakage, which happens regularly whenever YouTube changes its player.
**Fix:** Prefer itag 140 (AAC/MP4) until Opus/WebM is supported. Add retry/refresh on 403. Add a canary CI job that resolves a known video daily. Plan a quick-update path for extractor breakage. → TASK-157

## SRC-10 — Offline downloader is broken for network sources · P1
**Evidence:** `OfflineTrackDownloader::download_and_tag` calls the **blocking** `Read::read` on the stream inside an async fn (`crates/amp-core-engine/src/downloader.rs:40`). For progressive sources this returns `WouldBlock` almost immediately, which fails the download. It always names files `*.wav` regardless of container (`:30`), so MP3/M4A/WebM end up with a `.wav` extension and lofty then tags them wrongly. Filenames are only stripped of `/` and `\`, which allows `..`, `:`, `?`, control characters and reserved Windows names. It silently overwrites existing files. Nothing in any UI calls it.
**Fix:** → TASK-158

## SRC-11 — Bandcamp: no implementation · P1 (headline feature)
**Fix:** New `amp-adapter-bandcamp`: search (`bandcamp.com/search` or the internal API), album/track page parsing (`data-tralbum` JSON → `mp3-128` stream URLs), artwork, and optionally the fan collection. Respect free-stream limits. → TASK-159

## SRC-12 — SoundCloud: no implementation · P1 (headline feature)
**Fix:** New `amp-adapter-soundcloud`: `client_id` discovery, `api-v2` search, transcodings (progressive MP3 preferred, HLS fallback through a segment reader), and handling of the Go+ snippet limitation. → TASK-160

## SRC-13 — Soulseek: no server session · P1 (flagship differentiator)
**Evidence:** `SOULSEEK_SERVER_ADDR` is a constant and never connected to. The opcode enum mixes namespaces: server code 26 = FileSearch is correct, but `FileSearchResult = 9` and `TransferRequest = 40`/`41` are **peer**-message codes, and they share one `decode_packet` with server codes (`crates/amp-adapter-soulseek/src/lib.rs:24`). There is no Login (username, password, version, MD5 hash, minor version), no SetWaitPort, no ping, and no reconnect.
**Fix:** → TASK-162

## SRC-14 — Soulseek search & peer connections · P1
Needs: distributed and server search; peer `P`/`F`/`D` connection types; direct connect plus the firewalled `ConnectToPeer`/`PierceFirewall` path; zlib-compressed `FileSearchResponse` parsing; streaming results; attribute parsing (bitrate, duration, VBR, sample rate, bit depth). → TASK-163

## SRC-15 — Soulseek transfers · P1
Needs: `QueueUpload`, `TransferRequest`/`Response`, `PlaceInQueue`, an `F` connection with offset (resume), and progressive play from the precache while downloading. → TASK-164

## SRC-16 — Soulseek sharing (the "direct sharing character") · P1
Needs: a share index of the user's folders, responses to searches and browse requests, upload slots and queues, bandwidth limits, an ignore/ban list, and share-count reporting. The network's culture expects users to share; a leech-only client hurts the product's reputation and users' standing with peers. → TASK-165

## SRC-17 — P2P mesh has no networking or real trust · P1/P0 if shipped
**Evidence:** `verify_peer_handshake` trusts any peer with a non-empty token (`crates/amp-adapter-p2p/src/lib.rs:80`). There is no libp2p, no key pairs and no Noise, despite the docs. "BLAKE3/SHA256 content addressing" is a doc comment only.
**Decision needed:** Implement with libp2p (identity keys, Noise, Yamux, request-response, and pre-shared group key or invite-based allowlist), or remove from v1. → TASK-167

## SRC-18 — Spotify scope · P1 (decision)
librespot requires Spotify Premium, violates Spotify's Developer Terms for third-party playback clients, and is frequently broken by protocol changes. A "Spotify import → match on local/YouTube/Bandcamp/Soulseek" feature using the official Web API (metadata only) gives most of the user value with no legal exposure.
**Decision needed:** → TASK-168
