# AUDIT-2026-10-17 — multiplay Release-Readiness Audit

- **Date:** 2026-10-17
- **Commit audited:** `c85d74d` (main, clean tree)
- **Scope:** all 15 workspace crates (~16.5k LOC Rust), orchestration state (`.claude/`, `.nexus/`), scripts, Makefile, GitHub workflows.
- **Method:** full source read of every adapter, engine, runtime and UI crate; local `./scripts/verify_pure_rust.sh`, `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --check`; `cargo tree --target all` dependency analysis; GitHub Actions history (`gh run list`).
- **Goal measured against:** "a stable and releasable software product people will love."

## Executive verdict

**multiplay is a promising prototype, not a releasable product.** Local-file playback works on macOS through the GUI, and the code base is tidy, modular and lint-clean. But the orchestration system reports 105+ of 111 tasks as *Completed*, and the code does not back that up:

1. **Three of the five headline sources do not exist.** Spotify, Soulseek and the P2P mesh are in-memory `HashMap` mocks with no networking. Bandcamp and SoundCloud have no resolver at all. Only local files, YouTube and direct HTTP can play.
2. **Search doesn't work in either UI.** Both UIs build a `FilteredSearchCoordinator`, but neither calls it.
3. **Security: P0 remote-control exposure.** The GUI starts a WebSocket control server on `0.0.0.0:9002` on every launch. The server never checks the pairing token and has no Origin check, so anyone on the LAN, or any web page open in the user's browser, can control the player and make it open arbitrary file paths and URLs.
4. **The TUI library doesn't persist.** It lives in a fresh temp directory on every launch, and the TUI ships hard-coded demo tracks.
5. **The DSP isn't transparent, despite the audiophile claims.** With a flat, enabled EQ (the default), every sample above 0.85 FS is compressed by a soft clipper. A second limiter runs after it. The visualizer runs an FFT over interleaved stereo.
6. **Streaming correctness.** Network streams signal underruns with `WouldBlock`, which corrupts Symphonia's demuxer state mid-packet. Internet radio requests ICY metadata but never strips it, so metadata bytes are decoded as audio. The RAM buffer grows without bound on endless streams.
7. **CI is red and the "pure Rust" gate is unsound.** The verifier skips build-dependencies and non-host targets. On Linux, `cpal` links `libasound` through `alsa-sys`, which the project's own policy forbids. `cargo fmt --check` fails with 47 diffs. The release workflow copies a file that doesn't exist.
8. **About half the engine is dead code.** Nothing calls `ConfigStore`, the plugin registry, the downloader, the library scanner, smart shuffle, the CRDT jam queue, the BPM detector, WSOLA, headphone crossfeed or the keyring. The settings UI edits a config that is never saved or applied, except for crossfade, which is **on by default** and breaks gapless albums.

None of this is fatal. The hexagonal skeleton is the right shape, and the remediation below is ordered so that each milestone produces something users can actually run.

## Scorecard (0 = absent, 5 = release grade)

| Area | Score | One-line state |
| :--- | :---: | :--- |
| Architecture & modularity | 3 | Good crate topology; ports mostly unimplemented; playback logic duplicated in two UIs. |
| Audio output & transport | 2 | Plays on macOS; pause/stop/seek latency (~0.7 s ring), no format negotiation, no device recovery. |
| Decoding & streaming | 2 | Symphonia is used well for files; network underruns, unbounded buffers, no ICY, no range seek. |
| DSP / audiophile quality | 1 | Not transparent by default; resampler pads blocks mid-stream; loudness tags misparsed. |
| Sources: Local | 3 | Works; scanner is synchronous and symlink-unsafe, with no stable IDs. |
| Sources: YouTube / HTTP | 2 | Work in the happy path; fragile ID handling; downloader broken. |
| Sources: Soulseek / P2P / Spotify | 0 | Mocks only. |
| Sources: Bandcamp / SoundCloud | 0 | Absent (only enum variants and URL sniffing). |
| Search | 1 | Coordinator exists and is tested; not wired into any UI. |
| Persistence & config | 1 | GUI session JSON works; TUI DB is ephemeral; config never persisted; secrets in RAM. |
| Security & privacy | 0 | Unauthenticated LAN/browser-reachable remote control on by default. |
| UX (GUI) | 2 | Winamp look, docking, DnD, dialogs; no library browser, media keys, error feedback or seek polish. |
| UX (TUI) | 1 | Renders, but search, add files, seek, EQ and next/prev are missing; demo data. |
| Build / CI / release | 1 | Lint-clean locally; CI failing; release pipeline broken; no signing or installers. |
| Testing | 2 | ~145 tests pass locally; many tests need audio hardware, touch `$HOME` or bind `0.0.0.0`; no real-audio fixtures. |
| Docs, legal & product | 0 | No README, LICENSE or ToS stance; "WINAMP" trademark used in window titles. |
| Orchestration integrity | 1 | Status inflated; metrics in three places disagree; plan files point to tests that don't test what they claim. |

## Audit files

| File | Area | Findings |
| :--- | :--- | :--- |
| [AUDIT-01-orchestration-integrity.md](AUDIT-01-orchestration-integrity.md) | Plans and tasks vs. reality, policy | ORC-01 … ORC-06 |
| [AUDIT-02-architecture.md](AUDIT-02-architecture.md) | Hexagonal boundaries, duplication, dead code, concurrency model | ARC-01 … ARC-09 |
| [AUDIT-03-audio-pipeline.md](AUDIT-03-audio-pipeline.md) | Output, transport, decoder, streaming buffer | AUD-01 … AUD-14 |
| [AUDIT-04-dsp-audiophile.md](AUDIT-04-dsp-audiophile.md) | EQ, limiter, resampler, loudness, visualizer, crossfade | DSP-01 … DSP-08 |
| [AUDIT-05-sources-and-search.md](AUDIT-05-sources-and-search.md) | Every provider, search, downloader | SRC-01 … SRC-18 |
| [AUDIT-06-security-privacy.md](AUDIT-06-security-privacy.md) | Remote daemon, P2P trust, secrets, parsers | SEC-01 … SEC-08 |
| [AUDIT-07-persistence-data.md](AUDIT-07-persistence-data.md) | Library DB, session, config, playlists, paths | DAT-01 … DAT-07 |
| [AUDIT-08-ux-tui-gui.md](AUDIT-08-ux-tui-gui.md) | First run, daily use, TUI, GUI, DJ, OS integration | UX-01 … UX-10 |
| [AUDIT-09-build-ci-release.md](AUDIT-09-build-ci-release.md) | Pure-Rust gate, CI, release, supply chain | BLD-01 … BLD-07 |
| [AUDIT-10-testing-quality.md](AUDIT-10-testing-quality.md) | Test validity, hermeticity, fixtures, observability, budgets | QA-01 … QA-08 |
| [AUDIT-11-legal-product.md](AUDIT-11-legal-product.md) | Licensing, provider ToS, trademark, privacy | LEG-01 … LEG-06 |

## Severity legend

- **P0 — Release blocker.** Security exposure, data loss, or a core promise that is broken. Fix before any public build.
- **P1 — v1.0 required.** Users will hit it in normal use, or it contradicts the product pitch.
- **P2 — Should fix.** Quality, maintainability, or edge-case correctness.
- **P3 — Polish.**

## Finding → plan map

| Plan | Theme | Milestone | Addresses |
| :--- | :--- | :--- | :--- |
| PLAN-034 | Release truth reset & governance | M0 Stop the bleeding | ORC-*, BLD-01, LEG-01 |
| PLAN-035 | Remote control security hardening | M0 | SEC-01 … SEC-04 |
| PLAN-036 | Green CI on 3 OSes & hermetic tests | M0 | BLD-02 … BLD-05, QA-01, QA-02 |
| PLAN-037 | Unified core playback engine | M1 It plays, reliably | ARC-01 … ARC-05, AUD-01 … AUD-04 |
| PLAN-038 | Audio output device robustness | M1 | AUD-05 … AUD-08 |
| PLAN-039 | Decoder & progressive streaming correctness | M1 | AUD-09 … AUD-14, SRC-04 … SRC-06 |
| PLAN-040 | Audiophile DSP transparency | M2 Sounds right | DSP-01 … DSP-08 |
| PLAN-041 | Persistent library, config & secrets | M1 | DAT-01 … DAT-07, SEC-05 |
| PLAN-042 | Universal search wired end-to-end | M3 Finds everything | UX-01, SRC-07 |
| PLAN-043 | YouTube resolver & downloader hardening | M3 | SRC-08 … SRC-10 |
| PLAN-044 | Bandcamp & SoundCloud resolvers | M3 | SRC-11, SRC-12 |
| PLAN-045 | Real Soulseek client | M3 | SRC-13 … SRC-16 |
| PLAN-046 | P2P mesh & Spotify scope decisions | M3 | SRC-01, SRC-17, SRC-18, SEC-06 |
| PLAN-047 | UX: first run to daily driver | M4 Feels great | UX-02 … UX-10 |
| PLAN-048 | Observability & diagnostics | M4 | QA-04, QA-05 |
| PLAN-049 | Release engineering & distribution | M5 Ship | BLD-04, BLD-06, BLD-07, LEG-02 |
| PLAN-050 | Release-grade QA | M5 | QA-03, QA-06 … QA-08, SEC-07, ARC-06 |
| PLAN-051 | Codebase consolidation & architecture integrity | M5 (can run in parallel from M1) | ARC-07 … ARC-09 |
| PLAN-052 | Legal, privacy & provider compliance | M5 (start in M0) | LEG-03 … LEG-06, SEC-08 |

## What is genuinely good (keep it)

- Clean crate topology, with `#![forbid(unsafe_code)]` everywhere and a lint-clean workspace (`cargo clippy --all-targets`: 0 warnings).
- Symphonia, rubato, rustfft, redb, lofty, rtrb and ratatui/egui are the right library choices.
- The CPAL callback really is allocation-free and lock-free (`crates/amp-adapter-cpal/src/actor.rs:44`).
- Session snapshots are written atomically (temp file + rename), and so is `ConfigStore`.
- The Winamp-style docking and magnetic snapping work, and they carry a lot of the product's charm.
- The multi-source vision (local + YouTube + Bandcamp + Soulseek sharing in one player) is a genuinely differentiated product. It is worth finishing properly.
