# ADR-005: Pure-Rust Opus Decoder

- **Status:** Accepted (2026-10-31) — option 1, `rusty-opus`, chosen by the owner
- **Date:** 2026-10-31
- **Task:** `TASK-228` (`PLAN-058`: Universal Audio Format Support)
- **Supersedes:** [ADR-004](ADR-004-opus-codec-scope.md) (Opus hidden for lack of a pure-Rust decoder)

## Context

ADR-004 hid Opus until "an audited `#![forbid(unsafe_code)]` pure-Rust Opus decoder reaches
release maturity". Opus matters for local `.opus` files, WebM/MKA audio and YouTube's
Opus-only streams (itag 249–251). Four pure-Rust candidates exist on crates.io; none build C.

The project's rules (`.nexus/policy.json`) that apply:
- no bundled or vendored C (`verify_pure_rust.sh`); `#![forbid(unsafe_code)]` is required for
  *our* domain and core-engine crates, while dependencies (eframe, wgpu, cpal, …) may use
  `unsafe` internally;
- **active playback CPU < 1.0 %** of one core.

## Method

A throwaway spike decoded the 12 official RFC 8251 test vectors (`opus_testvectors-rfc8251`,
48 kHz stereo) with each candidate and compared against the reference decodings:
- **range check:** the decoder's range-coder state after each packet must equal the value the
  encoder recorded (the bit-exactness check `opus_demo` performs) — `opus-decoder` exposes it;
- **SNR** and share of **identical samples** versus the reference output;
- **speed** (release build, Apple M-series, one core), plus a realistic 30 s, 128 kbit/s
  music-like Ogg Opus file.

`opus_compare` (the official perceptual pass/fail tool, C) was not reproduced; SNR is a
stricter proxy for the music use case.

## Results

| Vector | `opus-decoder` 0.1.1: range / SNR / identical / speed | `rusty-opus` 1.0.2: SNR / identical / speed | `moosicbox_opus_native` 0.5.0 |
|---|---|---|---|
| 01 | 2147/2147 / 87.3 dB / 98.53 % / 16× | 106.1 dB / 99.98 % / 430× | fails (range decoder error) |
| 02 | 1185/1185 / ∞ / 100 % / 1570× | ∞ / 100 % / 1614× | fails (mono packet into stereo) |
| 03 | 998/998 / ∞ / 100 % / 1203× | ∞ / 100 % / 1258× | fails (mono packet into stereo) |
| 04 | 1265/1265 / ∞ / 100 % / 916× | ∞ / 100 % / 1074× | fails (mono packet into stereo) |
| 05 | 2037/2037 / 57.8 dB / 98.08 % / 12× | 100.7 dB / 100 % / 539× | fails (mono packet into stereo) |
| 06 | 1876/1876 / 47.6 dB / 96.91 % / 11× | 102.0 dB / 100 % / 513× | fails (mono packet into stereo) |
| 07 | 4186/4186 / 81.7 dB / 99.76 % / 17× | 99.9 dB / 100 % / 631× | fails (mono packet into stereo) |
| 08 | 1247/1247 / 78.6 dB / 99.61 % / 13× | 98.4 dB / 100 % / 666× | fails (no SILK resampling) |
| 09 | 1337/1337 / 81.6 dB / 99.77 % / 13× | 99.7 dB / 100 % / 137× | fails (no SILK resampling) |
| 10 | 1912/1912 / 60.6 dB / 84.49 % / 21× | 103.6 dB / 99.99 % / 472× | fails (frame size mismatch) |
| 11 | 553/553 / 86.1 dB / 96.27 % / 8× | 106.5 dB / 99.97 % / 552× | fails (range decoder error) |
| 12 | 1332/1332 / 42.9 dB / 95.95 % / 50× | 110.2 dB / 100 % / 1196× | fails (mono packet into stereo) |

**Realistic music (30 s, 128 kbit/s Ogg Opus):**
`opus-decoder` 8× realtime = **12.7 % of one core**; `rusty-opus` 458× realtime = **0.22 %**.

| | `opus-decoder` 0.1.1 | `rusty-opus` 1.0.2 | `moosicbox_opus_native` 0.5.0 | `opus-pure` 0.2.2 |
|---|---|---|---|---|
| License | MIT OR Apache-2.0 | BSD-3-Clause | MPL-2.0 | BSD-3-Clause |
| C / build scripts | none | none | none | none |
| `unsafe` | none, `#![forbid(unsafe_code)]` | 173 uses, all SIMD intrinsics (NEON/SSE/AVX), always on | none | 132 uses |
| Bitstream (range check) | 12/12 vectors exact | not exposed for the decoder | — | not run |
| Fidelity | 43–87 dB on CELT vectors | ~100 dB, ≥ 99.97 % identical | fails 11/12 | not run |
| CPU, music | 12.7 % ✗ (budget 1 %) | 0.22 % ✓ | — | not run |
| Maturity | 207 k downloads, last release 2026-03 | 74 k downloads, 2026-10 | 222 downloads | — |

`opus-pure` was not run: it has `unsafe` like `rusty-opus` with less adoption, so it cannot
beat it on either criterion. `moosicbox_opus_native` is not usable today.

## Options Considered

No candidate meets every criterion of ADR-004 **and** the CPU budget:

1. **Adopt `rusty-opus` (recommended).** Pure Rust, no C, near bit-exact, 0.22 % CPU. Requires
   amending ADR-004's criterion from "`#![forbid(unsafe_code)]` decoder" to "no C; `unsafe`
   only as SIMD intrinsics in an exactly pinned, license-checked dependency" — the same
   standing as wgpu, cpal and egui today. Our crates stay `#![forbid(unsafe_code)]`.
2. **Adopt `opus-decoder`.** Fully safe and bitstream-exact, but 12.7 % CPU for music breaks the
   1 % playback budget (worse on Raspberry Pi-class ARM), and fidelity is lower on CELT.
3. **No-go.** Keep Opus hidden (ADR-004 stays), re-check in 3 months; `opus-decoder` plans
   performance work.

## Decision

Adopt `rusty-opus` (option 1), pinned to `=1.0.2`. ADR-004's criterion becomes: **no C code;
`unsafe` only as SIMD intrinsics inside an exactly pinned, license-checked dependency**. Our own
crates stay `#![forbid(unsafe_code)]`. Re-evaluate `opus-decoder` when it gains performance work.

## Integration (TASK-229)

- Wrap the chosen decoder as a Symphonia `AudioDecoder` and register it in a custom codec
  registry inside `amp-adapter-decoder`, so Ogg Opus and WebM/MKA Opus use the existing path.
- Pin the exact version; keep the RFC 8251 vectors as an `#[ignore]`d conformance test and add
  Opus fixtures (Ogg, WebM; mono, stereo, 5.1) to the golden matrix.
- Honour pre-skip and output gain (RFC 7845), mark Opus `Playable` in `amp_domain::formats`,
  and let the YouTube resolver fall back to Opus streams.
