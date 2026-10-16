# multiplay — Pure-Rust Universal Audiophile Audio Player

Universal, ultra-lightweight, audiophile-grade, multi-source audio player written **100% in pure Rust**. Zero C/C++ dynamic or static linkage (no OpenSSL, SQLite, FFmpeg, libpulse, or libasound). Features a modular Winamp 1.x dense console, magnetic multi-window docking, audiophile DSP (rubato Sinc resampler, 10-band IIR biquad EQ, zero-alloc FFT spectrum visualizer), RAM precaching, pure-Rust embedded storage (`redb`), multi-source search, and stream resolvers (YouTube, Spotify, Bandcamp, SoundCloud, Soulseek, P2P mesh).

## Commands

```bash
# Verify pure-Rust toolchain compliance (Zero C build tools / libraries)
./scripts/verify_pure_rust.sh

# Build entire workspace
cargo build --workspace

# Run full test suite
cargo test --workspace

# Run TUI application
cargo run -p amp-app
```

## Hexagonal Architecture & Crate Topology

- `crates/amp-domain`: Pure domain models, entities, value objects (`Track`, `TrackId`, `AudioSource`, `SourceKind`, `SourceFilterMask`, `PlaybackState`). `#![forbid(unsafe_code)]`.
- `crates/amp-ports`: Driving (inbound) and driven (outbound) traits (`PlayerControlPort`, `PlaylistManagementPort`, `UniversalSearchPort`, `FilteredSearchPort`, `TrackDownloadPort`, `StreamResolverPort`, `ProviderSearchPort`, `AudioSinkPort`, `LibraryStoragePort`).
- `crates/amp-core-engine`: Playback orchestrator, `SharedPrecacheBuffer` (RAM precaching `Read + Seek`), `EventBus` CQRS dispatcher, `FilteredSearchCoordinator`, DSP chain (polyphase Sinc resampler, 10-band graphic EQ, 16-band FFT visualizer), offline downloader with Lofty metadata tagging, Winamp skin (`.wsz`) loader, plugin registry.
- `crates/amp-adapter-cpal`: Pure-Rust audio driver output (`cpal` + `rtrb` lock-free SPSC ring buffer 65,536 frames). Audio thread never allocates, locks, or performs syscalls.
- `crates/amp-adapter-decoder`: Symphonia pure-Rust format decoder adapter (FLAC, MP3, WAV, AAC, Vorbis, Opus).
- `crates/amp-adapter-http`: HTTP/HTTPS range-reader stream adapter (`reqwest` with `rustls`).
- `crates/amp-adapter-youtube`: YouTube stream resolver (`rusty-ytdl`).
- `crates/amp-adapter-spotify`: Spotify protocol adapter (`librespot-core`).
- `crates/amp-adapter-soulseek`: Soulseek binary protocol search and transfer client.
- `crates/amp-adapter-p2p`: Encrypted P2P private mesh (`libp2p` with Noise & Yamux).
- `crates/amp-adapter-storage`: Pure-Rust embedded key-value DB (`redb`), `lofty` tag extractor, local directory scanner/watcher.
- `crates/amp-adapter-tui`: Winamp 1.x dense console (`ratatui`), LED/VFD displays, 16-band mini spectrum analyzer, volume/balance, tape-deck transport buttons, magnetic multi-window docking manager (`WindowSlot`, `move_window_with_magnetic_snap`, tear-off), search modal with provider filter pills and source badges, detachable video mode view.
- `crates/amp-app`: Main runtime composition root, wiring ports and adapters, command dispatcher, signal handling.

## Nexus Orchestrator Guidelines

- All plans are stored in `.claude/plans/PLAN-NNN-*.md`
- All tasks are stored in `.claude/tasks/TASK-NNN-*.md`
- Master roadmap and execution index are maintained in `.claude/orchestrator-index.md`
- Machine state is tracked in `.claude/orchestrator.json`
- Policy constraints are enforced via `.nexus/policy.json`
- Always verify every task with `./scripts/verify_pure_rust.sh && cargo test --workspace` before marking completed.
