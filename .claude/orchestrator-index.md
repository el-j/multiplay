# Nexus Orchestrator Master Index: multiplay

**Universal Audiophile Pure-Rust Audio Player Engine & Modular Winamp TUI**  
*Tracking: 8 Plans | 25 Tasks | 12 Completed | Active: PLAN-005 (TASK-013)*

---

## Architecture Blueprint & Hexagonal Topology

```
                       DRIVING ADAPTERS (Primary Inbound)
       ┌─────────────────────────────────────────────────────────────┐
       │  ┌───────────────────┐  ┌────────────────┐  ┌────────────┐  │
       │  │ Ratatui TUI Shell │  │  egui GUI App  │  │  Headless  │  │
       │  │ (Winamp Modular)  │  │ (Desktop Node) │  │  Daemon/CLI│  │
       │  └─────────┬─────────┘  └───────┬────────┘  └─────┬──────┘  │
       └────────────┼────────────────────┼─────────────────┼─────────┘
                    │                    │                 │
                    ▼                    ▼                 ▼
       ┌─────────────────────────────────────────────────────────────┐
       │                        DRIVING PORTS                        │
       │   • PlayerControlPort           • PlaylistManagementPort    │
       │   • UniversalSearchPort         • FilteredSearchPort        │
       │   • TrackDownloadPort                                       │
       └──────────────────────────────┬──────────────────────────────┘
                                      │
                                      ▼
       ┌─────────────────────────────────────────────────────────────┐
       │                         DOMAIN CORE                         │
       │   • Entities: Track, Playlist, StreamSource, AudioMeta      │
       │   • Aggregates: PlaybackSession, AudioPipelineCoordinator   │
       │   • Services: SearchAggregator, DownloadJobManager          │
       │   • Value Objects: SampleFormat, BufferSpec, StreamChunk    │
       │   • Badges: SourceKind, SourceFilterMask                    │
       └──────────────────────────────┬──────────────────────────────┘
                                      │
                                      ▼
       ┌─────────────────────────────────────────────────────────────┐
       │                        DRIVEN PORTS                         │
       │   • AudioSinkPort               • StreamResolverPort        │
       │   • ProviderSearchPort          • LibraryStoragePort        │
       │   • AmpAudioPlugin              • DspFilterHook             │
       └──────────────────────────────┬──────────────────────────────┘
                    │                 │                │
                    ▼                 ▼                ▼
       ┌─────────────────────────────────────────────────────────────┐
       │                   DRIVEN ADAPTERS (Secondary Outbound)      │
       │  ┌───────────────────────┐  ┌────────────────────────────┐  │
       │  │ Hardware Output       │  │ Content & Stream Resolvers │  │
       │  │ • cpal (CoreAudio)    │  │ • reqwest-rustls (HTTP)    │  │
       │  │ • rtrb (Ring Buffer)  │  │ • rusty-ytdl (YouTube)     │  │
       │  ├───────────────────────┤  │ • librespot-core (Spotify) │  │
       │  │ Decoding & Formats    │  │ • Soulseek & P2P Mesh      │  │
       │  │ • symphonia (Pure)    │  ├────────────────────────────┤  │
       │  │   (FLAC, MP3, WAV,    │  │ Metadata & Database        │  │
       │  │    Vorbis, AAC, Opus) │  │ • redb (Pure Rust KV DB)   │  │
       │  │ • rubato Sinc DSP     │  │ • lofty (Tag Extractor)    │  │
       │  └───────────────────────┘  └────────────────────────────┘  │
       └─────────────────────────────────────────────────────────────┘
```

---

## Plans & Tasks Roadmap

### PLAN-001: Pure-Rust Toolchain, Domain Models & Hexagonal Port Contracts (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-001](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-001-cargo-workspace-and-toolchain.md) | Root Cargo Workspace & Pure-Rust Toolchain Constraints | Completed | architecture | `./scripts/verify_pure_rust.sh && cargo check --workspace` |
| [TASK-002](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-002-domain-models-and-entities.md) | Pure Domain Models, Entities & Source Badges (`amp-domain`) | Completed | architecture | `cargo test -p amp-domain` |
| [TASK-003](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-003-hexagonal-port-contracts.md) | Driving & Driven Hexagonal Port Traits (`amp-ports`) | Completed | architecture | `cargo test -p amp-ports` |

### PLAN-002: Real-Time Audio Engine, Precaching Buffer & Hardware Output (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-004](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-004-memory-precache-buffer.md) | In-Memory Precaching Stream Buffer (`SharedPrecacheBuffer`) | Completed | audio_engine | `cargo test -p amp-core-engine test_precache` |
| [TASK-005](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-005-cpal-hardware-audio-sink.md) | Pure-Rust CPAL Hardware Audio Sink & Lock-Free Ring Buffer | Completed | audio_engine | `cargo test -p amp-adapter-cpal` |
| [TASK-006](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-006-symphonia-decoder-engine.md) | Symphonia Pure-Rust Multi-Format Demuxer & Decoder | Completed | audio_engine | `cargo test -p amp-adapter-decoder` |

### PLAN-003: Audiophile DSP Pipeline, Sinc Resampling & FFT Visualizer (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-007](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-007-rubato-sinc-resampler.md) | Bit-Perfect Polyphase Sinc Resampler (`rubato`) | Completed | dsp_engineering | `cargo test -p amp-core-engine test_resampler` |
| [TASK-008](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-008-10-band-graphic-equalizer.md) | 10-Band Graphic Equalizer with IIR Peaking Biquad Filterbank | Completed | dsp_engineering | `cargo test -p amp-core-engine test_equalizer` |
| [TASK-009](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-009-fft-spectrum-visualizer.md) | Real-Time Zero-Allocation 16-Band FFT Spectrum Analyzer | Completed | dsp_engineering | `cargo test -p amp-core-engine test_visualizer` |

### PLAN-004: Embedded Storage (redb), CQRS Event Bus & Library Indexer (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-010](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-010-redb-storage-adapter.md) | Pure-Rust Embedded ACID Key-Value Storage Adapter (`redb`) | Completed | storage_and_state | `cargo test -p amp-adapter-storage` |
| [TASK-011](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-011-cqrs-event-bus-dispatcher.md) | CQRS Command-Event Bus & State Machine Orchestrator | Completed | storage_and_state | `cargo test -p amp-core-engine test_cqrs` |
| [TASK-012](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-012-library-scanner-lofty-tagger.md) | Local Library Filesystem Scanner with Pure-Rust Lofty Tagger | Completed | storage_and_state | `cargo test -p amp-adapter-storage test_scanner` |

### PLAN-005: Multi-Source Search Aggregator, Stream Resolvers & Downloader (Active)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-013](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-013-filtered-search-coordinator.md) | Parallel Multi-Source Search Coordinator with Filter Mask | Pending | integrations | `cargo test -p amp-core-engine test_search_coordinator` |
| [TASK-014](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-014-youtube-stream-resolver.md) | YouTube Pure-Rust Stream Resolver & Search Adapter (`rusty-ytdl`) | Pending | integrations | `cargo test -p amp-adapter-youtube` |
| [TASK-015](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-015-spotify-librespot-adapter.md) | Spotify Pure-Rust Stream Adapter (`librespot-core`) | Pending | integrations | `cargo test -p amp-adapter-spotify` |
| [TASK-016](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-016-offline-downloader-tagger.md) | Offline Stream Downloader Pipeline with Pure-Rust Lofty Tagging | Pending | integrations | `cargo test -p amp-core-engine test_downloader` |

### PLAN-006: Dense Winamp 1.x Faceplate & Multi-Window Magnetic Docking TUI
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-017](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-017-dense-winamp-faceplate.md) | Dense Winamp 1.x Faceplate Terminal Widget (`ratatui`) | Pending | frontend_tui | `cargo test -p amp-adapter-tui test_dense_console` |
| [TASK-018](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-018-magnetic-docking-window-manager.md) | Magnetic Multi-Window Docking & Snapping Window Manager | Pending | frontend_tui | `cargo test -p amp-adapter-tui test_docking` |
| [TASK-019](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-019-search-modal-source-badges.md) | Omnibar Search Modal with Filter Pills & Source Badges | Pending | frontend_tui | `cargo test -p amp-adapter-tui test_search_view` |
| [TASK-020](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-020-detachable-video-window.md) | Detachable Video Stream Canvas View | Pending | frontend_tui | `cargo test -p amp-adapter-tui test_video_view` |

### PLAN-007: Classic Winamp Skin (.wsz) Loader & Pure-Rust Plugin System
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-021](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-021-winamp-wsz-skin-loader.md) | Winamp Classic Skin (.wsz) Decompressor & Palette Extractor | Pending | extensibility | `cargo test -p amp-core-engine test_skin_loader` |
| [TASK-022](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-022-pure-rust-plugin-host.md) | Safe Pure-Rust Plugin Host & Capability Registry | Pending | extensibility | `cargo test -p amp-core-engine test_plugin_registry` |

### PLAN-008: P2P Mesh, Soulseek Protocol & Runtime Composition Root
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-023](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-023-soulseek-protocol-adapter.md) | Pure-Rust Soulseek Protocol Search & Transfer Adapter | Pending | system_composition | `cargo test -p amp-adapter-soulseek` |
| [TASK-024](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-024-encrypted-p2p-mesh-adapter.md) | Encrypted Private P2P Mesh Adapter (`libp2p`) | Pending | system_composition | `cargo test -p amp-adapter-p2p` |
| [TASK-025](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-025-runtime-composition-root.md) | Main Runtime Composition Root (`amp-app`) & System Integration | Pending | system_composition | `./scripts/verify_pure_rust.sh && cargo test --workspace` |

---

## Verification Pipeline & Quality Standards

1. **Pure-Rust Verification:** `./scripts/verify_pure_rust.sh` must succeed before any merge. Disallows `cc`, `cmake`, `pkg-config`, `openssl-sys`, `libloading`, `libsqlite3-sys`, and `ffmpeg-sys`.
2. **Audio Thread Real-Time Constraints:** CPAL data callback must be verified lock-free, allocation-free, and syscall-free.
3. **Hexagonal Boundaries:** Core domain crates never depend on concrete adapter crates.
