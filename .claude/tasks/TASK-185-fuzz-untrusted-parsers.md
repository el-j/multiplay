# TASK-185: Fuzz All Untrusted-Input Parsers & Add Size Limits

## Plan Reference
- Plan: [PLAN-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-050-release-grade-quality-assurance.md)
- Role: `security`
- Priority: **P2**
- Audit findings: `QA-07`, `SEC-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. `cargo-fuzz` targets: M3U/PLS/XSPF, `.wsz` loader (with entry/total size caps), ICY parser, Soulseek codec, remote JSON.
2. Nightly 10-minute fuzz job; crashes become regression tests.

## Acceptance Criteria (user-visible / measurable)
- Zip-bomb `.wsz` rejected with bounded memory; no fuzz crashes after 1 h each.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `fuzz/`
- `crates/amp-core-engine/src/skin.rs`

## Verification Command
`cargo +nightly fuzz run wsz -- -max_total_time=60`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-050-qa)
- Approach: property-based fuzzing with proptest (pure Rust) instead of cargo-fuzz, whose libFuzzer is C++ (native-code policy). Targets: `.wsz` skins (`amp-core-engine/tests/fuzz_skin.rs`), M3U/PLS/XSPF/CUE (`amp-adapter-storage/tests/fuzz_lists.rs`), ICY demuxer + radio playlists (`amp-adapter-http/tests/fuzz_icy.rs`), phone protocol (`amp-adapter-remote/tests/fuzz_protocol.rs`), Bandcamp answers (`amp-adapter-bandcamp/tests/fuzz_search.rs`), damaged audio incl. our DSD/WavPack/Opus code (`amp-adapter-decoder/tests/corrupt_files.rs`). 512 cases per property in CI; `.github/workflows/fuzz.yml` nightly with `FUZZ_CASES=20000`.
- Size limits: skins — 16 MiB file, 4 MiB per entry, 32 MiB unpacked, 512 entries (64 MiB zip bomb refused); playlists/CUE over 32 MiB skipped unread; ICY title history capped at 100.
- Bugs found and fixed: skin colour parser panicked on non-ASCII text (`=ଓᾶ`, regression test `non_ascii_colour_values_are_ignored`); damaged float WAV produced NaN/huge samples (decoder now outputs finite samples within ±2.0); Symphonia MS-ADPCM overflow panic on a damaged file — decode calls run in `amp_ports::contain_panic`, so a codec panic fails that file, not the playback thread; the crash hook logs contained panics instead of writing a crash report.
- 20,000-case local run: no further failures.
- Found on Linux CI: a bit flip in a DSDIFF header made our DSD parser allocate 64 GiB, which aborts the process on Linux (an abort cannot be contained; macOS overcommits lazily and passed). Fixed: header reads capped at 1 MiB, channels 1–8, block ≤ 64 KiB, DSD rate 2–50 MHz, saturating chunk arithmetic. `corrupt_files.rs` now runs under a 1 GiB capped allocator (`cap`, test-only) so this class fails the same way on every OS.
