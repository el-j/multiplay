# Nexus Orchestrator Master Index: multiplay

**Universal Audiophile Pure-Rust Audio Player Engine & Modular Winamp TUI**  
*Tracking: 52 Plans | 194 Tasks | 84 Verified Completed | Status: IN PROGRESS (Implementing PLAN-035: Remote Control Security Hardening)*

> **⚠ Release-readiness audit (2026-10-17):** a full code audit found that many tasks marked *Completed* below do not deliver their user-visible outcome (e.g. Soulseek/P2P/Spotify are in-memory mocks, search is unwired, CI is red). See [.claude/audits/README.md](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md). Status reconciliation of existing tasks is tracked in TASK-112 (to be done via the Nexus tools). The v1.0 roadmap (PLAN-034 … PLAN-052, TASK-112 … TASK-194) is listed under **Release Readiness Roadmap** below.





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
| [TASK-015](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-015-spotify-librespot-adapter.md) | Spotify Pure-Rust Stream Adapter (`librespot-core`) | Re-opened (Audit: ORC-01, SRC-01 -> PLAN-046/TASK-168) | integrations | `cargo test -p amp-adapter-spotify` |
| [TASK-016](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-016-offline-downloader-tagger.md) | Offline Stream Downloader Pipeline with Pure-Rust Lofty Tagging | Completed | integrations | `cargo test -p amp-core-engine test_downloader` |

### PLAN-006: Dense Winamp 1.x Faceplate & Multi-Window Magnetic Docking TUI (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-017](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-017-dense-winamp-faceplate.md) | Dense Winamp 1.x Faceplate Terminal Widget (`ratatui`) | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_dense_console` |
| [TASK-018](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-018-magnetic-docking-window-manager.md) | Magnetic Multi-Window Docking & Snapping Window Manager | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_docking` |
| [TASK-019](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-019-search-modal-source-badges.md) | Omnibar Search Modal with Filter Pills & Source Badges | Re-opened (Audit: ORC-01, UX-01 -> PLAN-042/TASK-153) | frontend_tui | `cargo test -p amp-adapter-tui test_search_view` |
| [TASK-020](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-020-detachable-video-window.md) | Detachable Video Stream Canvas View | Completed | frontend_tui | `cargo test -p amp-adapter-tui test_video_view` |

### PLAN-007: Classic Winamp Skin (.wsz) Loader & Pure-Rust Plugin System (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-021](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-021-winamp-wsz-skin-loader.md) | Winamp Classic Skin (.wsz) Decompressor & Palette Extractor | Completed | extensibility | `cargo test -p amp-core-engine test_skin_loader` |
| [TASK-022](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-022-pure-rust-plugin-host.md) | Safe Pure-Rust Plugin Host & Capability Registry | Completed | extensibility | `cargo test -p amp-core-engine test_plugin_registry` |

### PLAN-008: P2P Mesh, Soulseek Protocol & Runtime Composition Root (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-023](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-023-soulseek-protocol-adapter.md) | Pure-Rust Soulseek Protocol Search & Transfer Adapter | Re-opened (Audit: ORC-01, SRC-01, SRC-09 -> PLAN-045/TASK-162..166) | system_composition | `cargo test -p amp-adapter-soulseek` |
| [TASK-024](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-024-encrypted-p2p-mesh-adapter.md) | Encrypted Private P2P Mesh Adapter (`libp2p`) | Re-opened (Audit: ORC-01, SRC-01, SRC-16 -> PLAN-046/TASK-167) | system_composition | `cargo test -p amp-adapter-p2p` |
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
| [TASK-029](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-029-runtime-audio-playback-streaming-worker.md) | Dynamic Background Audio Playback Streamer in Runtime Composition Root | Completed | audio_engine | `cargo test -p amp-app --test e2e_pipeline` |
| [TASK-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-030-live-protocol-stream-resolvers.md) | Live Protocol Stream Handlers for Spotify, Soulseek TCP, and Encrypted P2P | Re-opened (Audit: ORC-01, SRC-01 -> PLAN-039, PLAN-043, PLAN-045) | integrations | `cargo test -p amp-adapter-spotify -p amp-adapter-soulseek` |
| [TASK-031](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-031-property-testing-and-mutation-suite.md) | Comprehensive Property-Based Testing (`proptest`) & Mutation Test Suite | Completed | qa_and_hardening | `cargo test --workspace` |
| [TASK-032](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-032-complete-incode-docstrings-and-audit-closure.md) | 100% In-Code Documentation & Architecture Reference Alignment Audit | Completed | architecture | `cargo doc --workspace --no-deps` |


### PLAN-011: Advanced Audio Playback Intelligence (Smart Shuffle, Gapless Crossfading & EBU R128 Normalization) (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-033](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-033-smart-balanced-shuffle-engine.md) | Smart Balanced Shuffle Engine & Constraint-Based Queue Manager | Completed | audio_engine | `cargo test -p amp-core-engine test_smart_shuffle` |
| [TASK-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-034-dual-pipeline-gapless-crossfader.md) | Dual-Pipeline Gapless Decoder Mixer & Real-Time Crossfader DSP | Completed | dsp_engineering | `cargo test -p amp-core-engine test_crossfade` |
| [TASK-035](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-035-pure-rust-ebu-r128-loudness-normalizer.md) | Pure-Rust EBU R128 Loudness Normalizer & Peak Limiter | Re-opened (Audit: ORC-01, DSP-05 -> PLAN-040/TASK-143) | dsp_engineering | `cargo test -p amp-core-engine test_loudness` |

### PLAN-012: Distributed Mesh Jam Session & Authenticated Mobile Remote Control (Experimental)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-036-collaborative-crdt-queue-state-machine.md) | Collaborative CRDT Queue & Lamport State Machine | Re-opened (Audit: ORC-01, UX-07 -> PLAN-050/TASK-187) | distributed_systems | `cargo test -p amp-domain test_crdt_queue` |
| [TASK-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-037-embedded-websocket-remote-daemon.md) | Pure-Rust Embedded WebSocket Remote Daemon (`amp-adapter-remote`) | Re-opened (Audit: ORC-01, SEC-01 -> PLAN-035/TASK-116..119) | integrations | `cargo test -p amp-adapter-remote` |
| [TASK-038](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-038-terminal-and-gui-qr-code-pairing.md) | Terminal & GUI QR Code Pairing System with Ephemeral Tokens | Re-opened (Audit: ORC-01, SEC-02 -> PLAN-035/TASK-117) | frontend_ui | `cargo test -p amp-adapter-remote test_pairing` |

### PLAN-013: Multi-Deck Professional DJ Engine & Universal Source Mixer (Experimental)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-039-multi-deck-channel-architecture-and-kill-eqs.md) | Multi-Deck Channel Architecture & 3-Band Kill Isolator EQs | Completed | audio_engine | `cargo test -p amp-core-engine test_dj_deck` |
| [TASK-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-040-pure-rust-realtime-bpm-beat-detector.md) | Pure-Rust Real-Time Transient BPM & Beat Grid Detector | Completed | dsp_engineering | `cargo test -p amp-core-engine test_bpm_detector` |
| [TASK-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-041-safe-wsola-time-stretcher-and-crossfader-mixer.md) | Safe WSOLA Time-Stretcher & Crossfader Transform Mixer | Completed | dsp_engineering | `cargo test -p amp-core-engine test_wsola_time_stretcher` |
| [TASK-042](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-042-multi-deck-dj-console-tui-and-gui-interface.md) | Multi-Deck Hardware-Style DJ Console Interface for TUI and GUI | Completed | frontend_ui | `cargo test -p amp-adapter-tui test_dj_console && cargo test -p amp-adapter-gui` |

### PLAN-014: Unified Configuration Management, Secure Credential Vault & Settings UI (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-043](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-043-hierarchical-toml-configuration-domain-schema.md) | Hierarchical TOML Configuration Domain Schema | Completed | architecture | `cargo test -p amp-domain test_config_schema` |
| [TASK-044](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-044-credential-vault-port-and-secure-keyring-adapter.md) | Driven Credential Vault Port & Secure Keyring Adapter | Re-opened (Audit: ORC-01, DAT-06 -> PLAN-041/TASK-151) | storage_and_state | `cargo test -p amp-adapter-storage test_keyring` |
| [TASK-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-045-atomic-config-storage-and-hot-reload-engine.md) | Atomic Config Storage Engine & Hot-Reload Dispatcher | Completed | storage_and_state | `cargo test -p amp-core-engine test_config_store` |
| [TASK-046](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-046-interactive-preferences-settings-modal.md) | Interactive Preferences Settings Modal Interface for TUI and GUI | Completed | frontend_ui | `cargo test -p amp-adapter-tui test_settings_view && cargo test -p amp-adapter-gui` |

### PLAN-015: Build Automation, CI/CD Pipeline & Cross-Platform Release Packaging (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-047-comprehensive-production-makefile.md) | Comprehensive Production Makefile | Completed | devops_and_build | `make help && make verify-pure-rust && make check` |
| [TASK-048](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-048-github-actions-ci-pipeline.md) | GitHub Actions Comprehensive CI Quality Pipeline | Completed | devops_and_ci | `test -f .github/workflows/ci.yml` |
| [TASK-049](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-049-github-actions-release-and-publishing-pipeline.md) | GitHub Actions Automated Cross-Platform Release & Publishing Pipeline | Completed | devops_and_release | `test -f .github/workflows/release.yml` |
| [TASK-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-050-end-to-end-verification-and-zero-gaps-audit.md) | End-to-End Verification, Zero-Gaps Audit, Coverage & Mutation Test Validation | Completed | qa_and_hardening | `make all && make verify-pure-rust` |

### PLAN-016: Modular Detachable & Magnetically Dockable Window System (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-051](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-051-magnetic-snapping-and-window-docking-geometry-engine.md) | Magnetic Snapping & Window Docking Geometry Engine | Completed | frontend_gui | `cargo test -p amp-adapter-gui test_docking` |
| [TASK-052](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-052-modular-windows-player-eq-playlist-settings-dj.md) | Modular Windows Implementation (Player, Equalizer, Playlist, Settings, DJ Console) | Completed | frontend_gui | `cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui` |
| [TASK-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-053-window-layout-persistence-and-session-restore.md) | Window Layout Persistence & Session Restore | Completed | frontend_gui | `cargo test -p amp-adapter-gui test_layout_persistence` |

### PLAN-017: Playlist Drag-and-Drop Ingestion & Multi-Format Local Media Scanner (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-054-native-drag-and-drop-ingestion-files-folders-urls.md) | Native OS Drag-and-Drop Ingestion for Audio Files, Folders & URLs | Completed | frontend_gui | `cargo test -p amp-adapter-gui test_drag_and_drop` |
| [TASK-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-055-metadata-extraction-and-playlist-tag-probing.md) | Pure-Rust Metadata Extraction & Playlist Item Probing | Completed | storage_and_state | `cargo test -p amp-adapter-storage test_probe_metadata` |
| [TASK-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-056-empty-playlist-onboarding-and-file-picker.md) | Empty Playlist Onboarding State & File Picker Integration | Completed | frontend_gui | `cargo test -p amp-adapter-gui test_playlist_empty_state` |

### PLAN-018: Real Symphonia Audio Playback Engine & Gapless Decoder Integration (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-057](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-057-real-time-audio-decoder-worker-and-stream-feeder.md) | Real-Time Audio Decoder Worker & Multi-Source Stream Feeder | Completed | audio_engine | `cargo test -p amp-adapter-gui test_real_audio_streaming` |
| [TASK-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-058-playback-progress-scrubbing-and-gapless-advancement.md) | Playback Progress Tracking, Scrubbing & Gapless Track Advancement | Completed | audio_engine | `cargo test -p amp-adapter-gui test_playback_tracking_and_seek` |

### PLAN-019: Audiophile Equalizer DSP Overhaul & Clipping Safeguard Engine (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-059](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-059-continuous-biquad-coefficient-morphing-and-state-preservation.md) | Continuous Biquad Filter Coefficient Morphing & State Preservation | Completed | dsp_engineering | `cargo test -p amp-core-engine test_biquad_state_preservation` |
| [TASK-060](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-060-preamp-gain-staging-headroom-and-soft-knee-limiter.md) | Preamp Gain Staging, Headroom Management & Soft-Knee Limiter DSP | Completed | dsp_engineering | `cargo test -p amp-core-engine test_eq_headroom_and_limiter` |

### PLAN-020: Orchestration Audit Remediation & Completion Re-baseline (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-061](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-061-orchestration-audit-remediation.md) | Re-baseline the orchestration status and close the completion claim gap | Completed | architecture | `./scripts/verify_pure_rust.sh && cargo check --workspace` |
| [TASK-062](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-062-domain-contract-alignment-and-error-standardization.md) | Domain Contract Alignment & Error Standardization | Completed | architecture | `cargo test -p amp-domain -p amp-ports` |

### PLAN-021: Elimination of Synthetic Stubs & Production Provider Hardening (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-063](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-063-spotify-adapter-stub-elimination-and-session-hardening.md) | Pure-Rust Spotify Adapter Stub Elimination & Session Hardening | Re-opened (Audit: ORC-01, SRC-01 -> PLAN-046/TASK-168) | integrations | `cargo test -p amp-adapter-spotify` |
| [TASK-064](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-064-soulseek-protocol-real-search-and-frame-state-machine.md) | Pure-Rust Soulseek Protocol Search & Message Frame State Machine | Re-opened (Audit: ORC-01, SRC-09 -> PLAN-045) | integrations | `cargo test -p amp-adapter-soulseek` |
| [TASK-065](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-065-p2p-mesh-cryptographic-handshake-and-block-transfer.md) | Pure-Rust P2P Mesh Cryptographic Handshake & Block Transfer | Re-opened (Audit: ORC-01, SRC-16 -> PLAN-046/TASK-167) | integrations | `cargo test -p amp-adapter-p2p` |
| [TASK-066](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-066-youtube-stream-resolver-live-buffering-hardening.md) | Pure-Rust YouTube Stream Resolver Live Buffering Hardening | Completed | integrations | `cargo test -p amp-adapter-youtube` |

### PLAN-022: End-to-End Audio Pipeline Integration & Cross-Platform Runtime Wiring (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-067](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-067-unified-playback-composition-root.md) | Unified Playback Composition Root & Hardware Audio Wiring | Completed | audio_engine | `cargo test -p amp-core-engine -p amp-adapter-cpal -p amp-adapter-decoder` |
| [TASK-068](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-068-unified-ui-runtime-integration.md) | Unified UI Runtime Integration Across CLI, TUI, and GUI | Completed | architecture | `cargo test -p amp-app -p amp-adapter-tui -p amp-adapter-gui` |
| [TASK-069](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-069-gapless-playback-seek-and-underrun-resilience.md) | Gapless Playback, Accurate Seeking & Underrun Resilience | Completed | audio_engine | `cargo test -p amp-core-engine test_gapless` |

### PLAN-023: Boss-File Refactoring & Modular Architectural Decomposition (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-070](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-070-decompose-app-main-monolith.md) | Decompose amp-app/src/main.rs Monolith into Modular Subsystems | Completed | architecture | `cargo check -p amp-app` |
| [TASK-071](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-071-decompose-gui-app-and-docking.md) | Decompose GUI App and Docking Modules into Focused Subsystems | Completed | gui_engineering | `cargo check -p amp-adapter-gui` |
| [TASK-072](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-072-decompose-cpal-and-decoder-adapters.md) | Decompose CPAL Sink and Symphonia Decoder into Modular Units | Completed | audio_engine | `cargo check -p amp-adapter-cpal -p amp-adapter-decoder` |

### PLAN-024: End-to-End Verification, Test Suite Hardening & CI Test Resilience (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-073](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-073-multi-format-e2e-audio-pipeline-test-harness.md) | Multi-Format E2E Audio Pipeline Test Harness | Completed | testing | `cargo test -p amp-core-engine test_e2e_playback_pipeline` |
| [TASK-074](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-074-memory-efficient-test-harness-and-ci-resilience.md) | Memory-Efficient Test Execution Harness & CI Resilience | Completed | testing | `./scripts/run_all_tests.sh` |
| [TASK-075](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-075-full-mutation-testing-pass-with-cargo-mutants.md) | Full Mutation Testing Execution & Mutation Gap Closure | Completed | testing | `make mutation-test` |

### PLAN-025: 100% In-Code Documentation, Complete Rustdoc Coverage & Zero-Warning Gate (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-076](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-076-domain-ports-engine-complete-rustdoc.md) | Domain, Ports & Core Engine Complete Rustdoc Documentation | Completed | documentation | `cargo doc -p amp-domain -p amp-ports -p amp-core-engine --no-deps` |
| [TASK-077](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-077-adapter-crates-complete-rustdoc.md) | Adapter Crates & Application Shells Complete Rustdoc Documentation | Completed | documentation | `cargo doc --workspace --no-deps` |
| [TASK-078](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-078-zero-warning-clippy-and-pure-rust-quality-gate.md) | Zero-Warning Clippy, 100% Pure-Rust & Release Quality Gate | Completed | architecture | `make all` |

### PLAN-026: Multi-Viewport Native Desktop Architecture (True Independent OS Windows) (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-079](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-079-multi-viewport-architecture-and-root-main-player.md) | Multi-Viewport Native Window Architecture & Root Main Player Window Configuration | Completed | gui_engineering | `cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui` |
| [TASK-080](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-080-satellite-windows-os-native-viewport-dispatch.md) | Satellite Windows OS Native Viewport Immediate Dispatch & Close Lifecycle Handling | Completed | gui_engineering | `cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui` |
| [TASK-081](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-081-cross-viewport-coordinate-tracking-and-docking.md) | Cross-Viewport Coordinate Tracking, Magnetic Snapping & Group Movement Synchronization | Completed | gui_engineering | `cargo test -p amp-adapter-gui test_docking` |
| [TASK-082](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-082-comprehensive-verification-and-quality-gate.md) | Comprehensive Testing, Verification, Zero Clippy Warnings, and Build Packaging | Completed | qa_and_hardening | `make all` |

### PLAN-027: Window Stability, Scaler Smoothness & Multi-Viewport Polish (Flicker-Free Industry-Grade Architecture) (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-083](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-083-eliminate-viewport-builder-perframe-commands-and-feedback-loop.md) | Eliminate ViewportBuilder Per-Frame Size/Position Commands and Positive Feedback Loop | Completed | gui_engineering | `cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui` |
| [TASK-084](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-084-drag-release-magnetic-snap-stabilization.md) | Drag-Release Magnetic Snap Stabilization and WindowServer Decoupling | Completed | gui_engineering | `cargo test -p amp-adapter-gui test_docking` |
| [TASK-085](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-085-conditional-repaint-scheduling-and-vsync-hardware-tuning.md) | Conditional Repaint Scheduling & VSync Hardware Tuning | Completed | gui_engineering | `cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui` |
| [TASK-086](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-086-comprehensive-verification-and-quality-gate.md) | Comprehensive Testing, Verification, Zero Clippy Warnings, and Build Packaging | Completed | qa_and_hardening | `make all` |

### PLAN-028: Bit-Perfect Audiophile Playback Pipeline & Hardware Sinc Resampling (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-087](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-087-live-polyphase-sinc-resampling-engine.md) | Live Polyphase Sinc Resampling Engine | Completed | dsp_engineering | `cargo test -p amp-adapter-gui -p amp-app` |
| [TASK-088](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-088-soft-knee-limiter-and-headroom-safeguard.md) | Soft-Knee Limiter & Headroom Safeguard | Completed | dsp_engineering | `cargo test -p amp-adapter-gui -p amp-core-engine` |
| [TASK-089](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-089-channel-matrix-and-downmixer.md) | ITU-R BS.775 Channel Matrix & Downmixer | Completed | audio_engine | `cargo test -p amp-adapter-decoder` |
| [TASK-090](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-090-lock-free-buffer-backpressure.md) | Adaptive Lock-Free Buffer Backpressure & Audio Thread Hardening | Completed | qa_and_hardening | `make all` |

### PLAN-029: Progressive Network Streaming & Universal Resolvers (YouTube & HTTP) (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-091](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-091-universal-multi-source-playback-dispatcher.md) | Universal Multi-Source Dispatcher in AudioPlaybackDriver | Completed | audio_engine | `cargo test -p amp-adapter-decoder -p amp-adapter-gui -p amp-app` |
| [TASK-092](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-092-progressive-async-stream-buffer.md) | Progressive Asynchronous Stream Buffer & YouTube Streaming Overhaul | Completed | integrations | `cargo test -p amp-adapter-youtube` |
| [TASK-093](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-093-direct-http-and-radio-resolver.md) | Pure-Rust Direct HTTP & Internet Radio Resolver | Completed | integrations | `cargo check --workspace && ./scripts/verify_pure_rust.sh` |
| [TASK-094](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-094-streaming-verification-harness.md) | Real Network Streaming Verification Harness | Completed | testing | `make all` |

### PLAN-030: Native File Dialogs, Playlist Persistence & Classic Winamp Skin (.wsz) Engine (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-095](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-095-native-os-file-and-directory-dialogs.md) | Native OS File & Directory Selection Dialogs | Completed | frontend_gui | `cargo test -p amp-adapter-gui && ./scripts/verify_pure_rust.sh` |
| [TASK-096](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-096-session-playlist-persistence-and-m3u8-support.md) | Session Playlist Persistence & M3U8 Export/Import | Completed | storage_and_state | `cargo test -p amp-adapter-gui -p amp-adapter-storage` |
| [TASK-097](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-097-full-winamp-skin-wsz-gui-rendering-engine.md) | Full Winamp Skin (.wsz) GUI Rendering Engine | Completed | frontend_gui | `cargo test -p amp-adapter-gui -p amp-core-engine` |
| [TASK-098](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-098-dual-oscilloscope-and-spectrum-visualizer-mode.md) | Dual Oscilloscope & Spectrum Visualizer Mode | Completed | dsp_engineering | `make all` |

### PLAN-031: Pro Live DJ Engine & Realtime Mixer (Experimental)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-099](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-099-dual-deck-live-playback-mixer-and-feeder.md) | Dual-Deck Live Playback Mixer & Feeder | Completed | audio_engine | `cargo test -p amp-core-engine -p amp-adapter-gui` |
| [TASK-100](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-100-live-3-band-kill-isolator-eq-and-wsola-pitch-shifting.md) | Live 3-Band Kill-Isolator EQ & WSOLA Pitch Shifting | Completed | dsp_engineering | `cargo test -p amp-core-engine test_dj` |
| [TASK-101](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-101-auto-bpm-detection-and-beatmatching-sync.md) | Auto-BPM Detection & Beatmatching Sync | Completed | audio_engine | `cargo test -p amp-core-engine -p amp-adapter-gui test_bpm` |
| [TASK-102](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-102-comprehensive-dj-engine-verification-harness.md) | Comprehensive DJ Engine Verification Harness | Completed | testing | `make all` |

### PLAN-032: Integrated Mobile Remote Daemon & Distributed Jam Session (Experimental)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-103](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-103-embedded-remote-daemon-autostart-and-qr-pairing.md) | Embedded Remote Daemon Auto-Start & QR Code Pairing in GUI | Completed | distributed_systems | `cargo test -p amp-adapter-remote -p amp-adapter-gui` |
| [TASK-104](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-104-embedded-pure-rust-mobile-web-controller.md) | Embedded Pure-Rust Mobile Web Controller | Completed | distributed_systems | `cargo test -p amp-adapter-remote` |
| [TASK-105](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-105-collaborative-jam-queue-and-mobile-sync.md) | Collaborative Jam Queue & Mobile Real-Time Sync | Completed | audio_engine | `cargo test -p amp-adapter-remote -p amp-adapter-gui test_remote` |
| [TASK-106](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-106-remote-daemon-and-web-client-verification-harness.md) | Remote Daemon & Web Client Verification Harness | Completed | testing | `make all` |

### PLAN-033: Audiophile Seamless Gapless Playback, Automix Crossfader & EBU R128 Normalization (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-107](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-107-lookahead-track-preloader-and-gapless-feeder.md) | Lookahead Track Preloader & Sample-Accurate Gapless Transition Feeder | Completed | audio_engine | `cargo test -p amp-adapter-gui -p amp-core-engine` |
| [TASK-108](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-108-configurable-equal-power-automix-crossfader.md) | Configurable Equal-Power Automix Crossfader | Completed | dsp_engineering | `cargo test -p amp-core-engine -p amp-adapter-gui` |
| [TASK-109](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-109-ebu-r128-and-replaygain-loudness-engine.md) | EBU R128 & ReplayGain Loudness Normalization Engine | Re-opened (Audit: ORC-01, DSP-05 -> PLAN-040/TASK-143) | dsp_engineering | `cargo test -p amp-domain -p amp-adapter-storage -p amp-core-engine -p amp-adapter-gui` |
| [TASK-110](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-110-audiophile-headphone-crossfeed-dsp.md) | Audiophile Headphone Crossfeed DSP | Completed | dsp_engineering | `cargo test -p amp-core-engine -p amp-adapter-gui` |
| [TASK-111](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-111-gapless-crossfade-and-dsp-verification-harness.md) | Gapless, Crossfade & DSP Pipeline Verification Harness | Completed | testing | `cargo test --test gapless_crossfade_pipeline && make all` |

---

## Release Readiness Roadmap — v1.0 (from AUDIT-2026-10-17)

Freeze on new feature plans until M2 is complete. Milestones run in order. PLAN-051 and PLAN-052 can run alongside them.

| Milestone | Outcome users feel | Plans |
| :--- | :--- | :--- |
| **M0 — Stop the bleeding** | Safe to run, honest docs, CI green | PLAN-034, PLAN-035, PLAN-036 |
| **M1 — It plays, reliably** | Instant pause/seek, robust streaming, persistent library & settings | PLAN-037, PLAN-038, PLAN-039, PLAN-041 |
| **M2 — Sounds right** | Bit-transparent by default, correct loudness, gapless | PLAN-040 |
| **M3 — Finds everything** | One search across library, YouTube, Bandcamp, SoundCloud, Soulseek | PLAN-042, PLAN-043, PLAN-044, PLAN-045, PLAN-046 |
| **M4 — Feels great** | Onboarding, library + artwork, media keys, full TUI, diagnostics | PLAN-047, PLAN-048 |
| **M5 — Ship** | Signed installers, release QA, legal & privacy, clean architecture | PLAN-049, PLAN-050, PLAN-051, PLAN-052 |

### PLAN-034: Release Truth Reset, Honest Status & Governance (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-112](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-112-reconcile-orchestrator-state-with-verified-reality.md) | Reconcile Orchestrator State with Verified Reality | Completed | architecture | `Manual review against `.claude/audits/README.md` scorecard` |
| [TASK-113](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-113-redefine-pure-rust-policy-and-sound-verifier.md) | Redefine the Pure-Rust Policy & Rewrite a Sound Verifier | Completed | architecture | `./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test` |
| [TASK-114](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-114-readme-license-and-honest-feature-matrix.md) | README, LICENSE, CONTRIBUTING & Honest Feature Matrix | Completed | architecture | `test -f README.md && test -f LICENSE && cargo metadata --format-version 1 \| grep -q '"license"'` |
| [TASK-115](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-115-evidence-based-definition-of-done.md) | Evidence-Based Definition of Done v2 | Completed | qa_and_hardening | `test -f docs/definition-of-done.md` |

### PLAN-035: Remote Control Daemon Security Hardening (In Progress · P0 · M0 — Stop the bleeding)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-116](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-116-remote-daemon-off-by-default-and-loopback-bind.md) | Remote Daemon Off by Default, Loopback Bind, Explicit LAN Opt-In | Pending | security | `cargo test -p amp-adapter-gui remote && ./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-117](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-117-enforce-pairing-session-secrets-and-origin-checks.md) | Enforce Pairing, Session Secrets, Origin Allow-List & Rate Limits | Pending | security | `cargo test -p amp-adapter-remote && cargo test -p amp-app --test remote_daemon_pipeline` |
| [TASK-118](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-118-constrain-remote-command-surface.md) | Constrain Remote Command Surface & Fix Transport Semantics | Pending | security | `cargo test -p amp-adapter-remote` |
| [TASK-119](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-119-remote-daemon-security-test-suite.md) | Remote Daemon Security Test Suite | Pending | qa_and_hardening | `cargo test -p amp-adapter-remote --test security` |

### PLAN-036: Green CI on macOS/Linux/Windows & Hermetic Test Suite (Pending · P0 · M0 — Stop the bleeding)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-120](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-120-fix-fmt-and-ci-system-dependencies.md) | Fix rustfmt Drift & Install Linux System Dependencies in CI | Pending | devops | `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --locked -- -D warnings` |
| [TASK-121](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-121-hardware-free-null-audio-sink-test-mode.md) | Hardware-Free Test Mode with Null/Capture Audio Sink | Pending | qa_and_hardening | `cargo test --workspace` |
| [TASK-122](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-122-hermetic-tests-no-home-no-network-no-public-bind.md) | Hermetic Tests: No $HOME Writes, No Network, No 0.0.0.0 Binds | Pending | qa_and_hardening | `HOME=$(mktemp -d) cargo test --workspace --offline` |
| [TASK-123](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-123-repair-release-workflow.md) | Repair the Release Workflow (Missing Files, Silent Failures) | Pending | devops | `act -j build-release (or a dry-run tag on a fork)` |
| [TASK-124](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-124-supply-chain-cargo-deny-audit-and-dependency-hygiene.md) | Supply Chain: cargo-deny, cargo-audit, Dependabot & Workspace Pinning | Pending | devops | `cargo deny check && cargo audit` |

### PLAN-037: Unified Core Playback Engine (PlayerEngine) & Transport Correctness (Pending · P0 · M1 — It plays, reliably)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-125](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-125-player-engine-implementing-driving-ports.md) | PlayerEngine: State Machine Implementing PlayerControlPort & PlaylistManagementPort | Pending | audio_engine | `cargo test -p amp-core-engine player` |
| [TASK-126](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-126-replace-duplicated-ui-audio-drivers-with-engine.md) | Replace Both UI Audio Drivers with PlayerEngine; Delete Duplicates | Pending | architecture | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-127](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-127-transport-flush-pause-stop-seek.md) | Transport Correctness: Epoch-Flushed Ring, Real Pause, No Lost Chunks | Pending | audio_engine | `cargo test -p amp-core-engine transport` |
| [TASK-128](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-128-output-clock-position-and-aligned-meters.md) | Output-Clock Playback Position & Output-Aligned Visualizer Tap | Pending | audio_engine | `cargo test -p amp-core-engine position` |
| [TASK-129](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-129-async-resolution-and-preloading-off-audio-thread.md) | Async Stream Resolution & Preloading Off the Audio Worker | Pending | audio_engine | `cargo test -p amp-core-engine preload` |
| [TASK-130](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-130-typed-errors-and-error-events.md) | Typed Port Errors & User-Visible Error Events | Pending | architecture | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-131](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-131-queue-semantics-next-prev-repeat-shuffle.md) | Queue Semantics: Next/Previous/Repeat/Shuffle Owned by the Engine | Pending | audio_engine | `cargo test -p amp-core-engine queue` |

### PLAN-038: Audio Output Device Robustness (Pending · P1 · M1 — It plays, reliably)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-132](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-132-sample-format-and-channel-negotiation.md) | Sample-Format & Channel-Count Negotiation (f32/i16/u16/i32, N channels) | Pending | audio_engine | `cargo test -p amp-adapter-cpal format` |
| [TASK-133](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-133-device-hotplug-and-stream-error-recovery.md) | Device Hot-Plug, Default-Device Change & Stream Error Recovery | Pending | audio_engine | `manual: plug/unplug test + cargo test -p amp-adapter-cpal recovery` |
| [TASK-134](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-134-honest-audio-sink-port.md) | Honest AudioSinkPort: Remove Dead push API, Real configure, Correct Latency | Pending | architecture | `./scripts/verify_pure_rust.sh && cargo test --workspace` |

### PLAN-039: Decoder & Progressive Streaming Correctness (Pending · P0 · M1 — It plays, reliably)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-135](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-135-blocking-precache-reader-with-timeouts-and-cancellation.md) | Blocking Precache Reader (Condvar) with Timeouts & Cancellation | Pending | audio_engine | `cargo test -p amp-ports precache && cargo test -p amp-adapter-http slow_server` |
| [TASK-136](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-136-bounded-memory-streaming-and-disk-cache.md) | Bounded-Memory Streaming: Live Ring Buffer & Disk Spill Cache | Pending | audio_engine | `cargo test -p amp-core-engine bounded_buffer` |
| [TASK-137](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-137-icy-metadata-and-radio-playlist-resolution.md) | ICY Metadata Demux & PLS/M3U/XSPF Radio Playlist Resolution | Pending | integrations | `cargo test -p amp-adapter-http` |
| [TASK-138](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-138-http-range-seeking.md) | HTTP Range Requests for Seekable Remote Files | Pending | integrations | `cargo test -p amp-adapter-http range` |
| [TASK-139](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-139-decoder-hardening-hints-reset-seek-opus.md) | Decoder Hardening: Probe Hints, ResetRequired, Accurate Seek, Opus/WebM | Pending | audio_engine | `cargo test -p amp-adapter-decoder` |

### PLAN-040: Audiophile DSP Transparency & Correctness (Pending · P1 · M2 — Sounds right)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-140](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-140-bit-transparent-default-path-and-null-test.md) | Bit-Transparent Default Path & CI Null Test | Pending | dsp_engineering | `cargo test -p amp-core-engine null_test` |
| [TASK-141](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-141-lookahead-oversampled-true-peak-limiter.md) | Lookahead, Oversampled True-Peak Limiter | Pending | dsp_engineering | `cargo test -p amp-core-engine limiter` |
| [TASK-142](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-142-streaming-correct-resampler.md) | Streaming-Correct Resampler (No Mid-Stream Padding, Per-Stream Instances) | Pending | dsp_engineering | `cargo test -p amp-core-engine resampler` |
| [TASK-143](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-143-correct-replaygain-and-r128-normalization.md) | Correct ReplayGain/R128 Normalization (+ Optional BS.1770 Analysis) | Pending | dsp_engineering | `cargo test -p amp-core-engine loudness && cargo test -p amp-adapter-storage` |
| [TASK-144](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-144-correct-log-spaced-mono-visualizer.md) | Visualizer: Mono Downmix, Log-Spaced Bands, dB Scaling | Pending | dsp_engineering | `cargo test -p amp-core-engine visualizer` |
| [TASK-145](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-145-gapless-first-crossfade-policy.md) | Gapless-First Crossfade Policy (Off by Default, Album-Aware, Rate-Matched) | Pending | dsp_engineering | `cargo test -p amp-core-engine gapless` |
| [TASK-146](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-146-perceptual-volume-taper-and-dither.md) | Perceptual Volume Taper & TPDF Dither | Pending | dsp_engineering | `cargo test -p amp-core-engine volume` |

### PLAN-041: Persistent Library, Configuration & Secrets (Pending · P0 · M1 — It plays, reliably)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-147](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-147-platform-app-paths-service-and-migration.md) | Platform AppPaths Service (config/data/cache/log) & Migration | Pending | storage_and_state | `cargo test -p amp-adapter-storage paths` |
| [TASK-148](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-148-durable-versioned-library-db-with-stable-ids.md) | Durable, Versioned Library DB with Stable Track IDs | Pending | storage_and_state | `cargo test -p amp-adapter-storage` |
| [TASK-149](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-149-background-incremental-scanner-and-watcher.md) | Background Incremental Scanner, Symlink-Safe, with File Watcher | Pending | storage_and_state | `cargo test -p amp-adapter-storage scanner` |
| [TASK-150](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-150-wire-config-store-and-apply-every-setting.md) | Wire ConfigStore: Persist Settings & Apply (or Remove) Every Option | Pending | storage_and_state | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-151](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-151-os-keychain-credential-vault.md) | OS Keychain Credential Vault (with Encrypted-File Fallback) | Pending | security | `cargo test -p amp-adapter-storage keyring` |
| [TASK-152](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-152-lossless-playlist-io-m3u-pls-xspf.md) | Lossless Playlist I/O: M3U/M3U8 (relative paths), PLS, XSPF | Pending | storage_and_state | `cargo test -p amp-adapter-storage playlist` |

### PLAN-042: Universal Search Wired End-to-End (Pending · P0 · M3 — Finds everything)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-153](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-153-async-omnibox-search-in-gui-and-tui.md) | Async Omnibox Search in GUI & TUI with Streaming Results | Pending | frontend_gui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-154](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-154-per-provider-status-and-error-surfacing.md) | Per-Provider Status, Timeouts & Error Surfacing | Pending | integrations | `cargo test -p amp-core-engine search` |
| [TASK-155](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-155-cross-source-ranking-and-dedupe.md) | Cross-Source Ranking & Deduplication | Pending | integrations | `cargo test -p amp-core-engine rank` |

### PLAN-043: YouTube Resolver & Offline Downloader Hardening (Pending · P1 · M3 — Finds everything)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-156](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-156-youtube-url-and-id-normalization.md) | YouTube URL/ID Normalization (watch, youtu.be, shorts, music, playlists) | Pending | integrations | `cargo test -p amp-domain source_url` |
| [TASK-157](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-157-youtube-format-selection-retry-and-canary.md) | YouTube Format Selection, URL Refresh, Retry & Daily Canary | Pending | integrations | `cargo test -p amp-adapter-youtube && gh workflow run canary` |
| [TASK-158](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-158-working-offline-downloader.md) | Working Offline Downloader (Async, Correct Containers, Safe Names, Progress) | Pending | integrations | `cargo test -p amp-core-engine downloader` |

### PLAN-044: Bandcamp & SoundCloud Resolvers (Pending · P1 · M3 — Finds everything)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-159](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-159-bandcamp-adapter.md) | New `amp-adapter-bandcamp`: Search, Album/Track Parsing, Streams, Purchase Links | Pending | integrations | `cargo test -p amp-adapter-bandcamp` |
| [TASK-160](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-160-soundcloud-adapter.md) | New `amp-adapter-soundcloud`: client_id Discovery, Search, Progressive/HLS Streams | Pending | integrations | `cargo test -p amp-adapter-soundcloud` |
| [TASK-161](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-161-scraper-contract-tests-and-canaries.md) | Recorded-Fixture Contract Tests & Live Canaries for Scraping Adapters | Pending | qa_and_hardening | `cargo test --workspace provider_contract` |

### PLAN-045: Real Soulseek Client: Session, Search, Transfers & Sharing (Pending · P1 · M3 — Finds everything)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-162](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-162-soulseek-server-session.md) | Soulseek Server Session: Login, Keepalive, Reconnect, Correct Message Namespaces | Pending | integrations | `cargo test -p amp-adapter-soulseek` |
| [TASK-163](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-163-soulseek-search-and-peer-connections.md) | Soulseek Search & Peer Connections (Direct + Firewalled) | Pending | integrations | `cargo test -p amp-adapter-soulseek search` |
| [TASK-164](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-164-soulseek-transfers-and-progressive-play.md) | Soulseek Transfers: Queue, F-Connection, Resume & Progressive Play | Pending | integrations | `cargo test -p amp-adapter-soulseek transfer` |
| [TASK-165](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-165-soulseek-sharing-and-uploads.md) | Soulseek Sharing: Share Index, Browse/Search Responses, Upload Slots & Limits | Pending | distributed_systems | `cargo test -p amp-adapter-soulseek share` |
| [TASK-166](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-166-soulseek-ui.md) | Soulseek UI: Account Settings, Transfer Manager, User Browse | Pending | frontend_gui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |

### PLAN-046: P2P Mesh & Spotify Scope Decisions; Remove Facades (Pending · P1 · M3 — Finds everything)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-167](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-167-adr-p2p-mesh-build-or-cut.md) | ADR: P2P Mesh — Implement with libp2p or Cut from v1 | Pending | architecture | `test -f docs/adr/ADR-002-p2p-mesh.md` |
| [TASK-168](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-168-adr-spotify-scope.md) | ADR: Spotify — Drop Playback, Offer Web-API Playlist Import & Cross-Source Matching | Pending | architecture | `test -f docs/adr/ADR-003-spotify.md` |
| [TASK-169](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-169-remove-facade-providers-from-production.md) | Remove/Feature-Gate Facade Providers from Production Builds | Pending | integrations | `./scripts/verify_pure_rust.sh && cargo test --workspace` |

### PLAN-047: UX: From First Run to Daily Driver (Pending · P1 · M4 — Feels great)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-170](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-170-remove-demo-data-show-real-track-info.md) | Remove Demo Data & Show Real Stream Info | Pending | frontend_tui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-171](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-171-first-run-onboarding.md) | First-Run Onboarding Wizard | Pending | frontend_gui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-172](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-172-library-browser-and-artwork.md) | Library Browser (Artists/Albums/Tracks) & Artwork | Pending | frontend_gui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-173](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-173-os-media-keys-and-now-playing.md) | OS Media Keys & Now Playing (MPRIS / macOS / SMTC) | Pending | frontend_gui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-174](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-174-tui-complete-player.md) | TUI as a Complete Player | Pending | frontend_tui | `cargo test -p amp-adapter-tui -p amp-app` |
| [TASK-175](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-175-gui-feedback-shortcuts-accessibility.md) | GUI Feedback, Shortcuts, Seek-on-Release & Accessibility | Pending | frontend_gui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-176](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-176-dj-mode-stabilization-or-experimental-flag.md) | DJ Mode: Experimental Flag, Own Engine Thread, Per-Deck Resampling | Pending | audio_engine | `cargo test -p amp-core-engine dj` |

### PLAN-048: Observability, Diagnostics & Measured Resource Budgets (Pending · P1 · M4 — Feels great)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-177](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-177-structured-logging-with-tracing.md) | Structured Logging with `tracing` to Rolling Log Files | Pending | qa_and_hardening | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-178](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-178-panic-hook-and-diagnostics-bundle.md) | Panic Hook, Crash Reports & 'Copy Diagnostics' | Pending | qa_and_hardening | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-179](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-179-measured-resource-budgets-benchmark-harness.md) | Measured Resource Budgets: Benchmark Harness & CI Tracking | Pending | qa_and_hardening | `cargo bench -p amp-core-engine` |

### PLAN-049: Release Engineering & Distribution (Pending · P1 · M5 — Ship)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-180](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-180-single-binary-versioning-and-changelog.md) | Single `multiplay` Binary, Versioning & CHANGELOG | Pending | devops | `cargo run -p amp-app -- --version` |
| [TASK-181](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-181-signed-installers-all-platforms.md) | Signed Installers: macOS DMG (notarized), Windows MSI, Linux AppImage/.deb/Flatpak | Pending | devops | `Release dry-run on tag in a fork` |
| [TASK-182](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-182-third-party-license-bundle.md) | Third-Party License Bundle & About Dialog | Pending | devops | `cargo about generate about.hbs > /dev/null` |
| [TASK-183](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-183-opt-in-update-checker-and-hotfix-channel.md) | Opt-In Update Checker & Provider Hotfix Channel | Pending | devops | `./scripts/verify_pure_rust.sh && cargo test --workspace` |

### PLAN-050: Release-Grade Quality Assurance (Pending · P1 · M5 — Ship)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-184](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-184-real-audio-fixture-corpus-and-golden-tests.md) | Real-Audio Fixture Corpus & Golden Decode Tests | Pending | qa_and_hardening | `cargo test --workspace golden` |
| [TASK-185](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-185-fuzz-untrusted-parsers.md) | Fuzz All Untrusted-Input Parsers & Add Size Limits | Pending | security | `cargo +nightly fuzz run wsz -- -max_total_time=60` |
| [TASK-186](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-186-soak-stress-and-failure-injection.md) | Soak, Stress & Failure-Injection Tests | Pending | qa_and_hardening | `cargo test --release -p amp-core-engine --test soak -- --ignored` |
| [TASK-187](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-187-contract-tests-real-mutation-runs-and-crdt-convergence.md) | Port Contract Tests, Real Mutation Runs & CRDT Convergence Fixes | Pending | qa_and_hardening | `cargo mutants --in-place -p amp-core-engine --file 'src/dsp/**'` |
| [TASK-188](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-188-manual-release-checklist-and-beta-program.md) | Manual Release Checklist & Beta Program | Pending | qa_and_hardening | `Manual` |

### PLAN-051: Codebase Consolidation & Architecture Integrity (Pending · P2 · M5 — Ship (parallelizable from M1))
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-189](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-189-wire-or-delete-dead-engine-modules.md) | Wire or Delete Dead Engine Modules (with ADR) | Pending | architecture | `./scripts/verify_pure_rust.sh && cargo test --workspace` |
| [TASK-190](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-190-restore-hexagonal-boundaries-single-composition-root.md) | Restore Hexagonal Boundaries & Single Composition Root | Pending | architecture | `./scripts/check_architecture.sh` |
| [TASK-191](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-191-message-passing-and-lock-free-parameters.md) | Replace Arc<Mutex> State Soup with Commands, Atomics & Triple Buffers | Pending | architecture | `cargo test -p amp-core-engine && cargo bench -p amp-core-engine` |

### PLAN-052: Legal, Privacy & Provider Compliance (Pending · P1 · M5 — Ship (start in M0))
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-192](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-192-provider-tos-review-and-opt-in-disclosures.md) | Provider ToS Review, Opt-In Disclosures & Download Policy | Pending | product | `cargo build -p amp-app --no-default-features` |
| [TASK-193](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-193-privacy-statement-and-network-inventory.md) | Privacy Statement & Network Activity Inventory | Pending | security | `test -f docs/privacy.md` |
| [TASK-194](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-194-trademark-safe-branding.md) | Trademark-Safe Branding (Remove 'WINAMP' from UI Titles) | Pending | frontend_gui | `./scripts/verify_pure_rust.sh && cargo test --workspace` |

---

## Verification Pipeline & Quality Standards

1. **Pure-Rust Verification:** `./scripts/verify_pure_rust.sh` must succeed before any merge. Disallows `cc`, `cmake`, `pkg-config`, `openssl-sys`, `libloading`, `libsqlite3-sys`, and `ffmpeg-sys`.
2. **Audio Thread Real-Time Constraints:** CPAL data callback must be verified lock-free, allocation-free, and syscall-free.
3. **Hexagonal Boundaries:** Core domain crates never depend on concrete adapter crates.
