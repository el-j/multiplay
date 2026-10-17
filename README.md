# multiplay

[![License: MPL-2.0](https://img.shields.io/badge/License-MPL_2.0-blue.svg)](https://opensource.org/licenses/MPL-2.0)
[![Pure Rust](https://img.shields.io/badge/Pure%20Rust-100%25-orange.svg)](#pure-rust-policy)

**multiplay** is a lightweight, audiophile-grade, multi-source audio player written in **pure Rust**.
It combines a modular, retro Winamp 1.x inspired dense faceplate with an audio engine built on strict hexagonal architecture, lock-free real-time audio threads, and pure-Rust audio decoding.

---

## Architecture & Principles

1. **Pure Rust:** Zero bundled third-party C/C++ libraries. Audio decoding (`symphonia`), embedded database (`redb`), cryptography & TLS (`rustls`), and DSP (`rubato`, `biquad`, `realfft`) are 100% pure Rust. Thin host OS platform bindings (CoreAudio, ALSA, WASAPI) are strictly audited in `scripts/pure_rust_allowlist.txt`.
2. **Hexagonal Modularity:** Domain core (`amp-domain`) and core engine (`amp-core-engine`) never import concrete adapters. All boundary crossings use strongly typed ports (`amp-ports`).
3. **Audio Thread Real-Time Safety:** The CPAL audio callback uses a lock-free single-producer single-consumer ring buffer (`rtrb`). It never allocates memory, waits on locks, makes system calls, or touches the filesystem.

---

## Honest Feature Matrix

Per the release readiness audit, this matrix represents verified capability:

| Feature / Subsystem | Status | Description |
| :--- | :--- | :--- |
| **Local Audio Playback** | **Stable** | FLAC, MP3, WAV, AAC, Vorbis, Opus via pure-Rust `symphonia` |
| **Hardware Output Sink** | **Stable** | Lock-free CPAL output on macOS (CoreAudio), Linux (ALSA), Windows (WASAPI) |
| **Audiophile Equalizer** | **Stable** | 10-band IIR peaking biquad filterbank with coefficient smoothing |
| **Sinc Resampling** | **Stable** | Polyphase bit-perfect sinc resampler via `rubato` |
| **Spectrum Analyzer** | **Stable** | Real-time 16-band FFT visualizer via `realfft` |
| **Embedded Database** | **Stable** | ACID key-value metadata & playlist persistence via pure-Rust `redb` |
| **Winamp Faceplate TUI** | **Stable** | Modular multi-window terminal interface (`ratatui`) with magnetic snapping |
| **Native Desktop GUI** | **Stable** | Multi-window desktop UI (`egui`/`eframe`) with dockable widgets |
| **HTTP Stream Resolver** | **Stable** | Range-reader HTTP/HTTPS stream resolving with icy-metadata |
| **YouTube Resolver** | **In Progress** | Format selection, stream buffering, and retry resilience (M3) |
| **Omnibar Search** | **In Progress** | UI overlay exists; async pipeline wiring to backends in progress (M3) |
| **Loudness Normalization**| **In Progress** | EBU R128 / ReplayGain tag extraction and true-peak limiter (M2) |
| **Soulseek P2P Search** | **Planned** | Binary protocol search, direct peer transfer, and share indexing (M3) |
| **Bandcamp & SoundCloud**| **Planned** | Track search, preview streams, and artist purchase page links (M3) |
| **DJ Engine & Mixer** | **Experimental** | Dual-deck playback, WSOLA pitch-shift, and auto-BPM detector |
| **Mobile Remote Control**| **Experimental** | Local WebSocket daemon with QR pairing (under security hardening) |

---

## Quickstart

### Prerequisites

- Rust 1.80+ (stable toolchain)
- Cargo

### Building

```bash
# Clone the repository
git clone https://github.com/rex-fab-alt/multiplay.git
cd multiplay

# Verify pure-Rust policy compliance & detection soundness
./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test

# Build the entire workspace
cargo build --workspace

# Run the test suite
cargo test --workspace
```

### Running

```bash
# Launch the Terminal (TUI) interface:
cargo run -p amp-app

# Launch the Desktop (GUI) interface:
cargo run -p amp-app -- --gui
```

---

## Pure-Rust Policy & Toolchain Verification

We verify that no vendored C/C++ libraries (such as OpenSSL, SQLite, or FFmpeg) enter the dependency tree across any target architecture:

```bash
./scripts/verify_pure_rust.sh
./scripts/verify_pure_rust.sh --self-test
```

All allowed OS-level platform `-sys` bindings are documented with explicit rationale in [`scripts/pure_rust_allowlist.txt`](scripts/pure_rust_allowlist.txt).

---

## Security & Disclosures

- **Remote Daemon:** By default, remote network control is disabled. Hardening and local loopback bind are tracked under `PLAN-035`.
- **Vulnerability Reporting:** See [`SECURITY.md`](SECURITY.md) for private vulnerability disclosure guidelines.

---

## Contributing & Governance

Contributions are welcome! Please read [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) before submitting pull requests. All changes must adhere to `#![forbid(unsafe_code)]` and the Definition of Done in [`docs/definition-of-done.md`](docs/definition-of-done.md).

---

## License

This project is licensed under the **Mozilla Public License Version 2.0 (MPL-2.0)**. See [`LICENSE`](LICENSE) for details.
