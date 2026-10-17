# Nexus Orchestrator Master Index: multiplay

**Universal Audiophile Pure-Rust Audio Player Engine & Modular Winamp TUI**  
*Tracking: 29 Plans | 94 Tasks | 90 Completed | 4 In Progress/Pending | Status: IN PROGRESS (Implementing PLAN-029)*





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

### PLAN-010: Zero-Gaps Closure, Live Playback Streaming & 100% Mutation Testing (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-029](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-029-runtime-audio-playback-streaming-worker.md) | Dynamic Background Audio Playback Streamer in Runtime Composition Root | Completed | audio_engine | `cargo test -p amp-app --test e2e_pipeline` |
| [TASK-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-030-live-protocol-stream-resolvers.md) | Live Protocol Stream Handlers for Spotify, Soulseek TCP, and Encrypted P2P | Completed | integrations | `cargo test -p amp-adapter-spotify -p amp-adapter-soulseek` |
| [TASK-031](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-031-property-testing-and-mutation-suite.md) | Comprehensive Property-Based Testing (`proptest`) & Mutation Test Suite | Completed | qa_and_hardening | `cargo test --workspace` |
| [TASK-032](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-032-complete-incode-docstrings-and-audit-closure.md) | 100% In-Code Documentation & Architecture Reference Alignment Audit | Completed | architecture | `cargo doc --workspace --no-deps` |


### PLAN-011: Advanced Audio Playback Intelligence (Smart Shuffle, Gapless Crossfading & EBU R128 Normalization) (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-033](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-033-smart-balanced-shuffle-engine.md) | Smart Balanced Shuffle Engine & Constraint-Based Queue Manager | Completed | audio_engine | `cargo test -p amp-core-engine test_smart_shuffle` |
| [TASK-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-034-dual-pipeline-gapless-crossfader.md) | Dual-Pipeline Gapless Decoder Mixer & Real-Time Crossfader DSP | Completed | dsp_engineering | `cargo test -p amp-core-engine test_crossfade` |
| [TASK-035](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-035-pure-rust-ebu-r128-loudness-normalizer.md) | Pure-Rust EBU R128 Loudness Normalizer & Peak Limiter | Completed | dsp_engineering | `cargo test -p amp-core-engine test_loudness` |

### PLAN-012: Distributed Mesh Jam Session & Authenticated Mobile Remote Control (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-036](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-036-collaborative-crdt-queue-state-machine.md) | Collaborative CRDT Queue & Lamport State Machine | Completed | distributed_systems | `cargo test -p amp-domain test_crdt_queue` |
| [TASK-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-037-embedded-websocket-remote-daemon.md) | Pure-Rust Embedded WebSocket Remote Daemon (`amp-adapter-remote`) | Completed | integrations | `cargo test -p amp-adapter-remote` |
| [TASK-038](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-038-terminal-and-gui-qr-code-pairing.md) | Terminal & GUI QR Code Pairing System with Ephemeral Tokens | Completed | frontend_ui | `cargo test -p amp-adapter-remote test_pairing` |

### PLAN-013: Multi-Deck Professional DJ Engine & Universal Source Mixer (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-039-multi-deck-channel-architecture-and-kill-eqs.md) | Multi-Deck Channel Architecture & 3-Band Kill Isolator EQs | Completed | audio_engine | `cargo test -p amp-core-engine test_dj_deck` |
| [TASK-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-040-pure-rust-realtime-bpm-beat-detector.md) | Pure-Rust Real-Time Transient BPM & Beat Grid Detector | Completed | dsp_engineering | `cargo test -p amp-core-engine test_bpm_detector` |
| [TASK-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-041-safe-wsola-time-stretcher-and-crossfader-mixer.md) | Safe WSOLA Time-Stretcher & Crossfader Transform Mixer | Completed | dsp_engineering | `cargo test -p amp-core-engine test_wsola_time_stretcher` |
| [TASK-042](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-042-multi-deck-dj-console-tui-and-gui-interface.md) | Multi-Deck Hardware-Style DJ Console Interface for TUI and GUI | Completed | frontend_ui | `cargo test -p amp-adapter-tui test_dj_console && cargo test -p amp-adapter-gui` |

### PLAN-014: Unified Configuration Management, Secure Credential Vault & Settings UI (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-043](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-043-hierarchical-toml-configuration-domain-schema.md) | Hierarchical TOML Configuration Domain Schema | Completed | architecture | `cargo test -p amp-domain test_config_schema` |
| [TASK-044](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-044-credential-vault-port-and-secure-keyring-adapter.md) | Driven Credential Vault Port & Secure Keyring Adapter | Completed | storage_and_state | `cargo test -p amp-adapter-storage test_keyring` |
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

### PLAN-021: Elimination of Synthetic Stubs & Production Provider Hardening (Completed)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-063](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-063-spotify-adapter-stub-elimination-and-session-hardening.md) | Pure-Rust Spotify Adapter Stub Elimination & Session Hardening | Completed | integrations | `cargo test -p amp-adapter-spotify` |
| [TASK-064](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-064-soulseek-protocol-real-search-and-frame-state-machine.md) | Pure-Rust Soulseek Protocol Search & Message Frame State Machine | Completed | integrations | `cargo test -p amp-adapter-soulseek` |
| [TASK-065](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-065-p2p-mesh-cryptographic-handshake-and-block-transfer.md) | Pure-Rust P2P Mesh Cryptographic Handshake & Block Transfer | Completed | integrations | `cargo test -p amp-adapter-p2p` |
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

### PLAN-030: Native File Dialogs, Playlist Persistence & Classic Winamp Skin (.wsz) Engine (In Progress)
| Task ID | Title | Status | Role | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| [TASK-095](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-095-native-os-file-and-directory-dialogs.md) | Native OS File & Directory Selection Dialogs | Pending | frontend_gui | `cargo test -p amp-adapter-gui && ./scripts/verify_pure_rust.sh` |
| [TASK-096](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-096-session-playlist-persistence-and-m3u8-support.md) | Session Playlist Persistence & M3U8 Export/Import | Pending | storage_and_state | `cargo test -p amp-adapter-gui -p amp-adapter-storage` |
| [TASK-097](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-097-full-winamp-skin-wsz-gui-rendering-engine.md) | Full Winamp Skin (.wsz) GUI Rendering Engine | Pending | frontend_gui | `cargo test -p amp-adapter-gui -p amp-core-engine` |
| [TASK-098](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-098-dual-oscilloscope-and-spectrum-visualizer-mode.md) | Dual Oscilloscope & Spectrum Visualizer Mode | Pending | dsp_engineering | `make all` |

---

## Verification Pipeline & Quality Standards

1. **Pure-Rust Verification:** `./scripts/verify_pure_rust.sh` must succeed before any merge. Disallows `cc`, `cmake`, `pkg-config`, `openssl-sys`, `libloading`, `libsqlite3-sys`, and `ffmpeg-sys`.
2. **Audio Thread Real-Time Constraints:** CPAL data callback must be verified lock-free, allocation-free, and syscall-free.
3. **Hexagonal Boundaries:** Core domain crates never depend on concrete adapter crates.
