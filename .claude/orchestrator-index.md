# Nexus Orchestrator Master Index: multiplay

**Universal Audiophile Pure-Rust Audio Player Engine & Modular Winamp TUI**  
*Tracking: 10 Plans | 32 Tasks | 28 Completed | Status: IN PROGRESS (Active: PLAN-010 / TASK-029)*

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

### PLAN-005: Multi-Source Search Aggregator, Stream Resolvers & Downloader (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-013](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-013-filtered-search-coordinator.md) | Parallel Multi-Source Search Coordinator with Filter Mask | Completed | integrations | `cargo test -p amp-core-engine test_search_coordinator` |
| [TASK-014](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-014-youtube-stream-resolver.md) | YouTube Pure-Rust Stream Resolver & Search Adapter (`rusty-ytdl`) | Completed | integrations | `cargo test -p amp-adapter-youtube` |
| [TASK-015](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-015-spotify-librespot-adapter.md) | Spotify Pure-Rust Stream Adapter (`librespot-core`) | Completed | integrations | `cargo test -p amp-adapter-spotify` |
| [TASK-016](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-016-offline-downloader-tagger.md) | Offline Stream Downloader Pipeline with Pure-Rust Lofty Tagging | Completed | integrations | `cargo test -p amp-core-engine test_downloader` |

### PLAN-006: Dense Winamp 1.x Faceplate & Multi-Window Magnetic Docking TUI (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-017](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-017-dense-winamp-faceplate.md) | Dense Winamp 1.x Faceplate Terminal Widget (`ratatui`) | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_dense_console` |
| [TASK-018](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-018-magnetic-docking-window-manager.md) | Magnetic Multi-Window Docking & Snapping Window Manager | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_docking` |
| [TASK-019](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-019-search-modal-source-badges.md) | Omnibar Search Modal with Filter Pills & Source Badges | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_search_view` |
| [TASK-020](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-020-detachable-video-window.md) | Detachable Video Stream Canvas View | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_video_view` |

### PLAN-007: Classic Winamp Skin (.wsz) Loader & Pure-Rust Plugin System (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-021](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-021-winamp-wsz-skin-loader.md) | Winamp Classic Skin (.wsz) Decompressor & Palette Extractor | Completed | extensibility | `cargo test -p amp-core-engine test_skin_loader` |
| [TASK-022](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-022-pure-rust-plugin-host.md) | Safe Pure-Rust Plugin Host & Capability Registry | Completed | extensibility | `cargo test -p amp-core-engine test_plugin_registry` |

### PLAN-008: P2P Mesh, Soulseek Protocol & Runtime Composition Root (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-023](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-023-soulseek-protocol-adapter.md) | Pure-Rust Soulseek Protocol Search & Transfer Adapter | Completed | system_composition | `cargo test -p amp-adapter-soulseek` |
| [TASK-024](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-024-encrypted-p2p-mesh-adapter.md) | Encrypted Private P2P Mesh Adapter (`libp2p`) | Completed | system_composition | `cargo test -p amp-adapter-p2p` |
| [TASK-025](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-025-runtime-composition-root.md) | Main Runtime Composition Root (`amp-app`) & System Integration | Completed | system_composition | `./scripts/verify_pure_rust.sh && cargo test --workspace` |

### PLAN-009: Native OS Desktop Windowing, Equalizer & Playlist Widgets, and Standalone Launcher (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-026](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-026-native-desktop-window-launcher-and-app-bundle.md) | Standalone Native Desktop Window Launcher, macOS Application Bundle & Scripts | Completed | frontend_gui | `./scripts/launch_desktop_window.sh && test -x Multiplay.command` |
| [TASK-027](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-027-docked-equalizer-and-playlist-editor-widgets.md) | Classic Winamp Equalizer & Playlist Editor Docked Widgets | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_equalizer_widget_rendering` |
| [TASK-028](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-028-pure-rust-gui-desktop-node-adapter.md) | Pure-Rust GUI Desktop Node Adapter (`amp-adapter-gui`) Architecture | Completed | frontend_gui | `./scripts/verify_pure_rust.sh && cargo check --workspace` |

### PLAN-010: Zero-Gaps Closure, Live Playback Streaming & 100% Mutation Testing (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-029](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-029-runtime-audio-playback-streaming-worker.md) | Dynamic Background Audio Playback Streamer in Runtime Composition Root | In Progress | audio_engine | `cargo test -p amp-app --test e2e_pipeline` |
| [TASK-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-030-live-protocol-stream-resolvers.md) | Live Protocol Stream Handlers for Spotify, Soulseek TCP, and Encrypted P2P | In Progress | integrations | `cargo test -p amp-adapter-spotify -p amp-adapter-soulseek` |
| [TASK-031](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-031-property-testing-and-mutation-suite.md) | Comprehensive Property-Based Testing (`proptest`) & Mutation Test Suite | In Progress | qa_and_hardening | `cargo test --workspace` |
| [TASK-032](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-032-complete-incode-docstrings-and-audit-closure.md) | 100% In-Code Documentation & Architecture Reference Alignment Audit | In Progress | architecture | `cargo doc --workspace --no-deps` |

---

## Verification Pipeline & Quality Standards

1. **Pure-Rust Verification:** `./scripts/verify_pure_rust.sh` must succeed before any merge. Disallows `cc`, `cmake`, `pkg-config`, `openssl-sys`, `libloading`, `libsqlite3-sys`, and `ffmpeg-sys`.
2. **Audio Thread Real-Time Constraints:** CPAL data callback must be verified lock-free, allocation-free, and syscall-free.
3. **Hexagonal Boundaries:** Core domain crates never depend on concrete adapter crates.
