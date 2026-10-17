# AUDIT-02 — Architecture

Part of [AUDIT-2026-10-17](README.md).

The crate topology follows the documented hexagonal design. The *runtime* architecture does not: the core engine is a toolbox of DSP parts, and each UI crate has grown its own private playback engine.

## ARC-01 — No playback engine exists in the core; driving ports are unimplemented · P0

**Evidence:** `PlayerControlPort`, `PlaylistManagementPort`, `UniversalSearchPort` and `TrackDownloadPort` (`crates/amp-ports/src/driving.rs`) have **zero implementations** in the workspace (`grep "impl .*Port for"`). Only `FilteredSearchPort` is implemented. The `EventBus` command channel is created in the TUI, but its receiver only handles `SetVolume`/`Stop` as UI echo (`crates/amp-app/src/runner.rs`). No engine consumes `PlayTrack`, `Seek*`, `AddToQueue` or `TriggerDownload`.

**Impact:** There is no single owner of playback state. Every UI and the remote daemon re-implement transport semantics, and they disagree (ARC-05, UX-09).

**Recommendation:** Build `amp-core-engine::player::PlayerEngine`. It owns the queue, transport state machine, decoder worker, DSP chain and sink, implements the driving ports, consumes `PlayerCommand` and emits `PlayerEvent`. UIs become thin. → TASK-125

## ARC-02 — The core engine depends on nothing it orchestrates · P1

**Evidence:** `amp-core-engine` has DSP, a shuffle engine, a crossfader, a gapless scheduler and a downloader, but no notion of a "current track", "queue" or "output". `GaplessTrackScheduler` and `ComfortSilenceGenerator` are exported but unused.

**Recommendation:** Same as ARC-01. Unused modules get wired or deleted (ARC-07). → TASK-125, TASK-189

## ARC-03 — Playback pipeline duplicated across `amp-app` and `amp-adapter-gui` · P1

**Evidence:**
- `crates/amp-app/src/track_loader.rs` and `crates/amp-adapter-gui/src/track_loader.rs` are byte-identical apart from comments and the thread name.
- `crates/amp-app/src/audio_driver.rs` (275 LOC) and `crates/amp-adapter-gui/src/audio_driver.rs` + `playback_worker.rs` (~450 LOC) are diverged copies. The GUI has gapless, crossfade, DJ, preamp and device switching; the TUI has none of them.
- Each crate builds its own private Tokio runtime via `OnceLock` (`track_loader.rs:10`).

**Impact:** Bugs get fixed twice or once. The TUI is permanently behind the GUI.

**Recommendation:** Delete both copies after TASK-125. Both UIs drive `PlayerEngine` through ports. → TASK-126

## ARC-04 — Stringly-typed errors across every port · P2

**Evidence:** `AudioSinkPort`, `LibraryStoragePort`, `CredentialVaultPort` and `TrackDownloadPort` all return `Result<_, String>`. `crates/amp-ports/src/error.rs` defines a rich `PortError`/`AudioSinkError`/`StorageError`/`VaultError` hierarchy that **nothing uses**. Callers match on substrings: `e.contains("WouldBlock")` in `crates/amp-adapter-gui/src/playback_worker.rs` and `crates/amp-app/src/audio_driver.rs`.

**Recommendation:** Adopt the existing typed errors on every port. Surface them to the UI as `PlayerEvent::Error { kind, message }`. → TASK-130

## ARC-05 — Transport semantics diverge between UI, engine and remote · P1

**Evidence:**
- The remote maps `Next` and `Previous` to `SeekRelative(±10 s)` (`crates/amp-adapter-remote/src/server.rs:150`). The GUI then *reinterprets* `SeekRelative(_, forward)` as next/previous track (`crates/amp-adapter-gui/src/app_render.rs`). `VoteSkip` becomes `SeekRelative(0, true)`.
- The TUI sets `playback_state = Playing` before any decoder has loaded, and `driver.playback_state()` never returns `Stopped`.
- A seek during pause is silently ignored in the GUI worker (the seek is only handled after the `is_playing` check).

**Recommendation:** Add explicit `Next`, `Previous` and `Skip` commands, with a single engine-owned state machine (`Stopped → Loading → Playing ⇄ Paused → Stopped`, plus `Buffering` and `Error`). → TASK-125, TASK-131

## ARC-06 — The CRDT jam queue doesn't converge · P2

**Evidence:** `crates/amp-domain/src/jam_crdt.rs:141`: `Upvote` increments a counter, so a re-delivered op double-counts. `:125`: an insert whose parent hasn't arrived yet is *appended*, so replicas that receive ops in different orders end up with different sequences. Upvote ops carry no per-peer set.

**Recommendation:** Store upvotes as `HashSet<peer_id>`. Buffer orphan inserts until the parent arrives (a causal delivery queue). Add proptest convergence tests over random op permutations and duplicates. Or cut the jam feature from v1 (TASK-167). → TASK-187

## ARC-07 — Large share of engine code is unreachable from any binary · P2

**Evidence:** Nothing in `amp-app`, `amp-adapter-gui` or `amp-adapter-tui` references `ConfigStore`, `PluginRegistry`, `OfflineTrackDownloader`, `LibraryScanner`, `CollaborativeQueue`, `SmartQueueManager`, `RealtimeBpmDetector`, `WsolaTimeStretcher`, `HeadphoneCrossfeed`, `KeyringStorageAdapter`, `GaplessTrackScheduler` or `ComfortSilenceGenerator`. `VideoWindowView` (TUI "detachable video") renders RGB buffers, but no video decoder exists.

**Recommendation:** For each module, decide **wire / feature-gate / delete**, and record the decision in an ADR. Dead code costs maintenance and inflates perceived completeness. → TASK-189

## ARC-08 — Adapter-to-adapter and adapter-to-engine coupling breaks the hexagon · P2

**Evidence:**
- `amp-adapter-remote` depends on `amp-core-engine` (for `EventBus` and `PlayerCommand`).
- `amp-adapter-gui` is a *composition root*: it depends on cpal, decoder, storage, youtube, soulseek, p2p, http and remote, and builds them itself (`crates/amp-adapter-gui/Cargo.toml`, `audio_driver.rs`).
- The GUI binary `multiplay-gui` lives inside the adapter crate.

**Recommendation:** Move `PlayerCommand` and `PlayerEvent` into `amp-ports`. Make `amp-app` the **only** composition root, with `multiplay` (GUI default) and `multiplay --tui` both built from it. UI adapters depend on ports only. → TASK-190

## ARC-09 — `Arc<Mutex<…>>` everywhere for real-time state · P2

**Evidence:** `AudioPlaybackDriver` exposes 15+ `pub Arc<Mutex<_>>` fields, which the UI thread and the audio worker lock on every tick (`volume`, `eq_gains`, `elapsed`, `current_track` …). The DJ tick clones the whole `DjAudioSharedState`, including `Track`s, every 512 frames (`crates/amp-adapter-gui/src/dj_playback.rs:26`). `audio_pusher.rs:24` silently drops a processed chunk when `producer.try_lock()` fails during a device switch.

**Recommendation:** Commands go in through a bounded channel. Parameters use atomics (`AtomicU32` bit-cast floats) or a triple buffer. Meters come out through a lock-free snapshot. No public mutable fields. → TASK-191
