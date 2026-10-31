# TASK-228: ADR-005 Spike: Pure-Rust Opus Decoder (Go/No-Go)

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `codec_engineering`
- Priority: **P1**
- Depends on: — (supersedes [ADR-004](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/adr/ADR-004-opus-codec-scope.md) if accepted)

## Context
ADR-004 (2026-10-27) hid Opus because no pure-Rust decoder was in the tree. Candidates on
crates.io as of 2026-10-31:

| Crate | Version | License | Activity | Note |
|---|---|---|---|---|
| `opus-decoder` | 0.1.1 | MIT OR Apache-2.0 | 207 k downloads, last release 2026-03 | Claims RFC 8251 conformance, no unsafe, no FFI |
| `rusty-opus` | 1.0.2 | BSD-3-Clause | 74 k downloads, released 2026-10-17 | Encoder + decoder, SILK/CELT/hybrid, SIMD |
| `moosicbox_opus` + `moosicbox_opus_native` | 0.5.0 | MPL-2.0 | small, active (2026-10-16) | Ships as a Symphonia codec adapter |
| `opus-pure` | 0.2.2 | BSD-3-Clause | — | Opus + Ogg encapsulation |

## Objectives
1. Run each candidate against the official RFC 8251 / RFC 6716 test vectors
   (`opus_testvectors`), recording pass/fail and quality metric per vector.
2. Check policy: `verify_pure_rust.sh` (no C, no build scripts compiling native code),
   `unsafe` usage (`cargo geiger` or grep), license against `deny.toml`, MSRV.
3. Measure decode speed (realtime factor at 48 kHz stereo, release build) and allocations
   per packet; check packet-loss concealment and all modes (SILK, CELT, hybrid).
4. Write `docs/adr/ADR-005-opus-decoder.md`: chosen crate (or no-go with re-check date),
   integration route (Symphonia `CodecRegistry` registration vs. own Ogg/WebM demux).

## Acceptance Criteria (user-visible / measurable)
- ADR-005 accepted with a conformance table for every candidate.
- Chosen decoder passes all RFC 8251 vectors and decodes ≥ 20× realtime on CI hardware.
- Pure-Rust verification passed with the candidate added (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `docs/adr/ADR-005-opus-decoder.md` (new)
- spike crate or example under `crates/amp-adapter-decoder/examples/` (removed after the decision)

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo deny check && cargo test -p amp-adapter-decoder`

## Manual Verification & Evidence
- **Build / Run Command:** spike example decoding the test vectors
- **Observed Behavior:** Conformance and speed table in the ADR.
- **Evidence Links:** ADR-005, terminal log.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31)
- Spike (throwaway, outside the workspace) decoded all 12 RFC 8251 vectors with `opus-decoder` 0.1.1, `rusty-opus` 1.0.2 and `moosicbox_opus_native` 0.5.0, plus a 30 s 128 kbit/s music-like Ogg Opus file. Full tables in [ADR-005](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/adr/ADR-005-opus-decoder.md).
- `opus-decoder`: range check exact on 12/12 vectors (19,973 packets), `#![forbid(unsafe_code)]`, but 43–87 dB SNR on CELT vectors and 12.7 % of one core for music (policy budget 1 %).
- `rusty-opus`: ~100 dB SNR, ≥ 99.97 % identical samples, 0.22 % CPU; 173 `unsafe` uses, all SIMD intrinsics; BSD-3-Clause (allowed by `deny.toml`).
- `moosicbox_opus_native`: fails 11/12 vectors. `opus-pure`: not run (unsafe like rusty-opus, less adoption).
- ADR-005 is **Proposed**: no candidate meets ADR-004's `forbid(unsafe)` criterion and the CPU budget together; owner decision needed (recommendation: adopt `rusty-opus`).
