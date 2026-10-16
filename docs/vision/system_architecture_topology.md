# Architecture & Hexagonal Topology: Pure-Rust Universal Audio Player

## 1. Executive Summary & Design Principles

This project defines an ultra-lightweight, audiophile-grade, multi-source audio player written **100% in pure Rust**. The application guarantees zero native C/C++ dynamic or static linkage (avoiding OpenSSL, FFmpeg, SQLite, libpulse, or libasound). It bridges local audio collections, network shares (SMB/NFS), and online streaming providers (YouTube, Spotify, Bandcamp, SoundCloud) into a unified playlist and library engine.

### Core Non-Negotiables
1. **Pure Rust Toolchain (`#![forbid(unsafe_code)]` in Domain & Adapters):** Every component compiles cleanly via standard `cargo build` on macOS, Linux, and Windows. No host C compiler (`clang`, `gcc`) or CMake dependency is permitted.
2. **Deterministic Resource Overhead:** Steady-state idle CPU consumption $< 0.2\%$; active playback CPU $< 1.0\%$ on Apple Silicon / modern x86_64. Base memory footprint $\le 35\text{ MB}$ (excluding dynamic RAM prefetch buffers).
3. **Audio Thread Isolation:** The output callback from the OS audio driver (`cpal`) must never allocate memory, perform system calls, access file systems, or wait on locks.
4. **Hexagonal Modularity:** Online sources, audio sinks, metadata indexers, and user interfaces interact solely via abstract driving and driven ports. Adding a source requires zero modifications to playback logic.

---

## 2. Hexagonal Topology (Ports and Adapters)

```
                            DRIVING ADAPTERS (Primary)
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
          │   • UniversalSearchPort         • TrackDownloadPort         │
          └──────────────────────────────┬──────────────────────────────┘
                                         │
                                         ▼
          ┌─────────────────────────────────────────────────────────────┐
          │                         DOMAIN CORE                         │
          │   • Entities: Track, Playlist, StreamSource, AudioMeta      │
          │   • Aggregates: PlaybackSession, AudioPipelineCoordinator   │
          │   • Services: SearchAggregator, DownloadJobManager          │
          │   • Value Objects: SampleFormat, BufferSpec, StreamChunk    │
          └──────────────────────────────┬──────────────────────────────┘
                                         │
                                         ▼
          ┌─────────────────────────────────────────────────────────────┐
          │                        DRIVEN PORTS                         │
          │   • AudioOutputSinkPort         • StreamResolverPort        │
          │   • CodecDecoderPort            • ProviderSearchPort        │
          │   • LocalStoragePort            • MetadataTagPort           │
          └──────────────────────────────┬──────────────────────────────┘
                       │                 │                │
                       ▼                 ▼                ▼
          ┌─────────────────────────────────────────────────────────────┐
          │                   DRIVEN ADAPTERS (Secondary)               │
          │  ┌───────────────────────┐  ┌────────────────────────────┐  │
          │  │ Hardware Output       │  │ Content & Stream Resolvers │  │
          │  │ • cpal (CoreAudio)    │  │ • reqwest-rustls (HTTP)    │  │
          │  │ • rtrb (Ring Buffer)  │  │ • rusty-ytdl (YouTube)     │  │
          │  ├───────────────────────┤  │ • librespot-core (Spotify) │  │
          │  │ Decoding & Formats    │  ├────────────────────────────┤  │
          │  │ • symphonia (Pure)    │  │ Metadata & Database        │  │
          │  │   (FLAC, MP3, WAV,    │  │ • redb (Pure Rust KV DB)   │  │
          │  │    Vorbis, AAC, Opus) │  │ • id3, lofty, metaflac     │  │
          │  └───────────────────────┘  └────────────────────────────┘  │
          └─────────────────────────────────────────────────────────────┘
```

---

## 3. Cargo Workspace Directory Structure

To enforce boundary separation, the workspace is split into isolated crates:

```
├── Cargo.toml
├── Cargo.lock
├── crates
│   ├── amp-domain          # Zero-dependency domain models and rules
│   ├── amp-ports           # Trait definitions for driving and driven ports
│   ├── amp-core-engine     # Playback orchestrator, ring buffers, precaching
│   ├── amp-adapter-cpal    # Pure-Rust audio driver output
│   ├── amp-adapter-decoder # Symphonia pure-Rust format decoder adapter
│   ├── amp-adapter-http    # HTTP/HTTPS range-reader stream adapter (reqwest+rustls)
│   ├── amp-adapter-youtube # YouTube stream resolver (rusty-ytdl)
│   ├── amp-adapter-spotify # Spotify protocol adapter (librespot-core)
│   ├── amp-adapter-storage # Pure-Rust embedded key-value DB (redb)
│   ├── amp-adapter-tui     # Winamp-style Ratatui terminal application
│   └── amp-app             # Main runtime composition root
```

---

## 4. Root `Cargo.toml` Manifest

```toml
[workspace]
members = [
    "crates/amp-domain",
    "crates/amp-ports",
    "crates/amp-core-engine",
    "crates/amp-adapter-cpal",
    "crates/amp-adapter-decoder",
    "crates/amp-adapter-http",
    "crates/amp-adapter-youtube",
    "crates/amp-adapter-spotify",
    "crates/amp-adapter-storage",
    "crates/amp-adapter-tui",
    "crates/amp-app",
]
resolver = "2"

[workspace.dependencies]
# Concurrency & Async
tokio = { version = "1.40", features = ["sync", "rt-multi-thread", "macros", "fs", "io-util"] }
crossbeam-channel = "0.5"
rtrb = "0.3" # Real-Time Ring Buffer: Lock-free SPSC queue

# Pure-Rust Audio & Codecs
cpal = "0.15"
symphonia = { version = "0.5", default-features = false, features = ["mp3", "flac", "pcm", "vorbis", "aac", "isomp4", "ogg"] }

# Pure-Rust Network & Stream Resolvers
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream"] }
rustls = { version = "0.23", default-features = false, features = ["ring"] }
rusty-ytdl = { version = "0.7", default-features = false, features = ["rustls-tls"] }
librespot-core = { version = "0.6", default-features = false }
librespot-playback = { version = "0.6", default-features = false }

# Pure-Rust Persistence & Metadata
redb = "2.1"
lofty = "0.21"

# Terminal Interface
ratatui = "0.28"
crossterm = "0.28"

# Utilities
async-trait = "0.1"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.10", features = ["v4", "serde"] }
```

---

## 5. Pure-Rust Verification Strategy

To guarantee that no transitive dependency silently introduces a C/C++ compiler or dynamic library requirement:

1. **Cargo Configuration (`.cargo/config.toml`):**
   ```toml
   [env]
   # Prevent OpenSSL from attempting to find or build C libraries
   OPENSSL_NO_VENDOR = "1"
   ```

2. **Automated CI Validation:**
   The CI pipeline runs:
   ```bash
   cargo tree --edges no-build --prefix depth | grep -E "(libloading|cc|cmake|pkg-config)"
   ```
   If any toolchain builders or native C-bindings are discovered in production targets, the build fails immediately.