# AUDIT-06 — Security & Privacy

Part of [AUDIT-2026-10-17](README.md).

## SEC-01 — Remote control server auto-starts on all interfaces · P0
**Evidence:** `MultiplayGuiApp::new()` calls `remote.start_daemon()` unconditionally (`crates/amp-adapter-gui/src/app_state.rs:47`). It binds `0.0.0.0:9002` (`crates/amp-adapter-gui/src/remote_manager.rs:85`). There is no setting to disable it, and the user is not told. Unit tests that construct `MultiplayGuiApp` also open this listener.
**Impact:** Every launch exposes a control surface to the whole LAN: café Wi-Fi, office networks, hotel networks.
**Fix:** Off by default. When enabled, bind `127.0.0.1` unless the user explicitly picks "Allow devices on my network". Show a visible indicator while it's listening. → TASK-116

## SEC-02 — Pairing token is generated and displayed but never checked · P0
**Evidence:** `PairingManager` lives only in `RemoteGuiManager`. `RemoteDaemon::handle_connection` (`crates/amp-adapter-remote/src/server.rs:68`) accepts every WebSocket upgrade and dispatches every parsed `RemoteAction`. `validate_and_consume` is called only in tests. The token is 8 hex characters (32 bits) from a UUID prefix, which is weak even once it is enforced, and it has no brute-force throttling.
**Fix:** Validate the token on upgrade (query or first frame). Exchange it for a long-lived random session secret (≥128 bits) that the client stores. Throttle failures. Cap connections. → TASK-117

## SEC-03 — Cross-site WebSocket hijacking from any web page · P0
**Evidence:** The `Origin` header isn't checked. Any website the user opens can `new WebSocket("ws://127.0.0.1:9002")` and send `{"SetVolume":1.0}` or `EnqueueTrack`. There is no CSRF protection on the HTTP routes.
**Fix:** Allow-list the origin as the daemon's own served origin, and require the session secret. → TASK-117

## SEC-04 — Remote `EnqueueTrack` accepts arbitrary `AudioSource` · P0
**Evidence:** `RemoteAction::EnqueueTrack(Box<Track>)` deserializes a full `Track`, including `AudioSource::LocalFile { path }` and `DirectHttp { url }`. The GUI pushes it straight into the playlist (`app_render.rs`) and the engine opens it. Combined with SEC-01 to SEC-03, a remote attacker can make the host read arbitrary local files through the decoder, and make it issue HTTP requests to internal addresses (SSRF against `169.254.169.254`, router admin pages and so on). Playlist session persistence then saves the attacker's entries.
**Fix:** Remotes may enqueue only by library `TrackId` or by a search-result token the host issued. URL enqueue requires host-side confirmation. → TASK-118

## SEC-05 — Credentials are not stored securely (or at all) · P1
**Evidence:** `KeyringStorageAdapter` is an in-memory `RwLock<HashMap>` (`crates/amp-adapter-storage/src/keyring.rs:63`). `get_secret` returns a plain `String` copy, which defeats `SecretString`'s zero-on-drop. Real Soulseek and SoundCloud credentials will need durable, protected storage.
**Fix:** The `keyring` crate (macOS Keychain, Windows Credential Manager, Secret Service on Linux) behind `CredentialVaultPort`, with an encrypted-file fallback (pure-Rust AEAD such as `chacha20poly1305`, key derived from the OS keychain or a user passphrase). → TASK-151

## SEC-06 — P2P "trust" model is absent · P1 if shipped
**Evidence:** `crates/amp-adapter-p2p/src/lib.rs:80`, where any non-empty `auth_token` makes a peer trusted.
**Fix:** Covered by the decision in TASK-167. If shipped: Ed25519 identity, invite-code pairing, Noise XX, and an allowlist persisted per group.

## SEC-07 — Untrusted-input parsers lack limits · P2
**Evidence:**
- The `.wsz` skin loader calls `read_to_end` on every zip entry with no size cap (`crates/amp-core-engine/src/skin.rs:114`). Skins are downloaded from the internet, so a zip bomb exhausts memory.
- The remote peeks 1 KB and does substring HTTP routing. Future Soulseek frame parsing will take attacker-controlled lengths (the existing `decode_packet` reads `length` but never bounds-checks it against the buffer).
- The library scanner recurses through symlinks, so a symlink loop overflows the stack (`scanner.rs:48`).
**Fix:** Cap entry and total sizes, cap frame lengths, and track visited inodes. Fuzz every parser (TASK-185). → TASK-149, TASK-185

## SEC-08 — Undisclosed network activity · P2
**Evidence:** `detect_local_ip()` opens a UDP socket "towards" `8.8.8.8:80` (no packet is sent, but it is surprising in a firewall prompt). On macOS the `0.0.0.0` bind triggers the "accept incoming connections?" dialog on first launch with no context. The HTTP User-Agent claims `Multiplay/1.0`.
**Fix:** Enumerate interfaces with the `if-addrs` crate. Document every outbound connection in a privacy page. Use the real version in the UA. → TASK-193
