# ADR-004: Opus Codec Scope in Pure-Rust Audio Pipeline

- **Status:** Superseded by [ADR-005](ADR-005-opus-decoder.md) (2026-10-31): Opus plays via `rusty-opus`
- **Date:** 2026-10-27
- **Task:** `TASK-139` (`PLAN-039`: Decoder Hardening: Probe Hints, ResetRequired, Accurate Seek, Opus/WebM)
- **Audit Findings:** `AUD-13`

## Context

`multiplay` enforces a strict `#![forbid(unsafe_code)]` and 100% pure-Rust dependency policy (`./scripts/verify_pure_rust.sh`), forbidding bundled C/C++ codec libraries such as `libopus` (`audiopus_sys`, `opus-sys`).

Symphonia `0.6` provides native pure-Rust decoders for MP3, FLAC, PCM/WAV, Ogg Vorbis, and AAC/ISO-MP4, which are enabled and verified in `amp-adapter-decoder`. However, Symphonia `0.6` does not ship a built-in pure-Rust RFC 6716 Opus decoder, and no production-grade, `#![forbid(unsafe_code)]` pure-Rust Opus decoder crate is locked in the workspace.

Previously, `.opus` appeared in the local scanner extension list and file dialog filter list even though attempting to play an `.opus` file failed inside Symphonia with an opaque codec error (`AUD-13`).

## Decision

1. **Do not advertise `.opus` in file pickers or local library scanning:** Remove `"opus"` from `SUPPORTED_EXTENSIONS` in `amp-adapter-gui::file_dialog` and `amp-adapter-storage::scanner`.
2. **Explicit, clean rejection in `amp-adapter-decoder`:** If a user or stream explicitly passes an `.opus` file extension or `audio/opus` MIME hint to `AudioStreamDecoder`, return a clear, actionable `DecodeError::UnsupportedFormat` explaining that Opus is not supported in the pure-Rust Symphonia codec set (referencing `ADR-004`).
3. **Future Re-evaluation:** Once Symphonia ships its upstream pure-Rust `symphonia-codec-opus` crate (or an audited `#![forbid(unsafe_code)]` pure-Rust Opus decoder reaches release maturity), `.opus` and WebM/Opus demuxing can be enabled behind a golden decode test suite.
