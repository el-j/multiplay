# multiplay

[![License: MPL-2.0](https://img.shields.io/badge/License-MPL_2.0-blue.svg)](LICENSE)
[![CI](https://github.com/el-j/multiplay/actions/workflows/ci.yml/badge.svg)](https://github.com/el-j/multiplay/actions/workflows/ci.yml)

**multiplay** is a multi-source audio player written in Rust: local files, internet radio and web
sources today, with YouTube, Bandcamp, SoundCloud and Soulseek sharing on the roadmap. It has a
desktop GUI with a classic dense faceplate, dockable windows and `.wsz` skin support, plus a
terminal (TUI) front end.

> **Status: pre-release (0.1).** Not yet ready for everyday use. A release-readiness audit
> ([`.claude/audits/README.md`](.claude/audits/README.md)) drives the v1.0 roadmap; the matrix
> below reflects what actually works today, not what is planned.

---

## Feature Matrix

**Stable** = works end to end on a release build. **Beta** = works with known defects.
**Experimental** = incomplete, may be removed. **Planned** = not implemented.

| Feature | Status | Notes |
| :--- | :--- | :--- |
| Local playback in the GUI (FLAC, MP3, WAV, AAC/M4A, Ogg Vorbis) | **Beta** | Pure-Rust decoding via `symphonia`. Opus is **not** supported yet. Output is f32 stereo only. |
| Desktop GUI (`egui`/`eframe`) | **Beta** | Docking windows, playlist, EQ, `.wsz` skins. Errors are not yet shown in the UI. |
| Terminal UI (`ratatui`) | **Experimental** | Cannot add files, seek or skip; library is not persisted between launches. |
| 10-band graphic EQ | **Beta** | Applies soft clipping even when flat (audit DSP-01); not transparent yet. |
| Sample-rate conversion (`rubato` sinc) | **Beta** | Partial blocks are zero-padded, which can click at track ends. |
| Spectrum visualizer | **Beta** | Analyses interleaved samples (inaccurate bands). |
| HTTP streams & internet radio | **Beta** | ICY metadata is not stripped from the audio; buffering is unbounded. |
| Playlist/session persistence (GUI) | **Beta** | Session file in `~/.multiplay`. Settings are not saved. |
| Loudness normalization (ReplayGain / R128) | **Planned** | Tags are not applied yet. |
| Universal search across sources | **Planned** | Search UI exists but no search is executed. |
| YouTube | **Experimental** | Resolver exists; playback of the selected stream format is unverified. |
| Bandcamp, SoundCloud | **Planned** | Not implemented. |
| Soulseek search, download & sharing | **Planned** | Current crate is an in-memory placeholder with no networking. |
| Spotify | **Not supported** | Dropped for now ([ADR-007](docs/adr/ADR-007-drop-spotify.md)); old playlist entries say why. |
| Private P2P mesh | **Planned** | Current crate is a placeholder; scope under review. |
| DJ mode | **Beta** | The main window becomes a two-deck console (Ctrl/Cmd+J) without a gap in the music. Beat-synced decks with key lock, prelisten in headphones (split cable or a second device), loops and hot cues, 3-band EQ with kills, filter, limiter, waveforms with beat markers, and auto mix through a playlist assigned to decks. Covered by automated tests on all platforms; a live release-build session is still to be recorded. |
| Phone remote control (Wi-Fi, QR pairing) | **Experimental** | Off by default; see [Security](#security). |

---

## Download

Beta builds for macOS (universal `.dmg`), Windows (`.msi` installer or `.zip`) and Linux
(x86_64 and ARM64: `.deb`, `.AppImage` or `.tar.gz`) are on the [Releases page](https://github.com/el-j/multiplay/releases).
Each release's notes explain how to install it; [CHANGELOG.md](CHANGELOG.md) lists what changed.
Maintainers: see [docs/RELEASING.md](docs/RELEASING.md).

## Privacy

No telemetry, no accounts, no update checks. multiplay only goes online when you search, play a
stream or link, or turn on remote control. [docs/privacy.md](docs/privacy.md) lists every
connection and what is sent.

## Quickstart

Requires a recent stable Rust toolchain. On Linux you also need the ALSA development package
(`libasound2-dev` on Debian/Ubuntu) and the usual X11/Wayland development headers for `eframe`.

```bash
git clone https://github.com/el-j/multiplay.git
cd multiplay

# Desktop GUI
cargo run --release -p amp-adapter-gui --bin multiplay-gui

# Terminal UI (experimental)
cargo run --release -p amp-app
```

### Development checks

```bash
./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Native-Code Policy

multiplay does not bundle or vendor third-party C/C++ libraries (no OpenSSL, SQLite, FFmpeg or
aws-lc). Decoding (`symphonia`), storage (`redb`) and DSP (`rubato`, `realfft`) are pure Rust.
It does link the audio, windowing and graphics libraries your operating system provides
(CoreAudio, ALSA, WASAPI, Wayland, …). TLS uses `rustls`, whose `ring` crypto provider compiles
C/assembly primitives: that is the one documented exception.

Every crate that links or builds native code must be justified in
[`scripts/pure_rust_allowlist.txt`](scripts/pure_rust_allowlist.txt);
[`scripts/verify_pure_rust.sh`](scripts/verify_pure_rust.sh) enforces this for all targets.

---

## Security

The phone remote is **off by default**. When enabled in Settings → Remote it listens on
`127.0.0.1` only, unless you also allow devices on your network. Phones pair by scanning a
single-use QR code; cross-site and DNS-rebinding requests are refused, failed attempts are
throttled, and **remotes never make the host open anything without approval**: a phone
can only *request* a track (one from your library, an allow-listed YouTube link, or a search),
and you accept or reject it in the Remote window (📱). Requests are rate limited per phone,
file paths, IP addresses and unknown sites are refused, and nothing ever auto-plays. Phone
uploads are a separate opt-in for the current session only (50 MB limit, decode-checked,
deleted when remote control stops). Builds before
this change started an unauthenticated server on `0.0.0.0:9002`. Please update.
Report vulnerabilities privately as described in [`SECURITY.md`](SECURITY.md).

---

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md), [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) and the
[Definition of Done](docs/definition-of-done.md) before opening a pull request.

## License

[Mozilla Public License 2.0](LICENSE). Every download includes `THIRD_PARTY_LICENSES.html`
with the licenses of the open-source crates multiplay is built on (generate it with
`cargo about generate about.hbs -o THIRD_PARTY_LICENSES.html`). "Winamp" is a trademark of its respective owner;
multiplay is not affiliated with it and only reads classic `.wsz` skin files.
