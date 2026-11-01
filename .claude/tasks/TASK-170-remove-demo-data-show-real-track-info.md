# TASK-170: Remove Demo Data & Show Real Stream Info

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_tui`
- Priority: **P1**
- Audit findings: `UX-02` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-126

## Objectives
1. Delete hard-coded sample tracks; empty-state onboarding text instead.
2. Faceplate shows actual sample rate, bit depth/bitrate, channels, and BIT-PERFECT/DSP state from engine events.

## Acceptance Criteria (user-visible / measurable)
- No literal track titles or `44.1/320` constants remain in UI crates.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/app_state.rs`
- `crates/amp-adapter-tui/src/dense_console.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-047-ux)
- TUI: the two hard-coded demo tracks and the "Aphex Twin - Polynomial-C" title are gone; empty playlist with "No track loaded. Press J to search." The format badge (`DenseConsole::audio_info`) is only drawn while a stream decodes, from `PlayerEngine::stream_info()` and the track's tags (no `44.1/320` constants).
- Engine: `PlayerEngine::stream_info()` → `StreamInfo { source_rate, output_rate, bit_perfect }`, recorded per chunk by the worker (two atomics) from the decoder rate and `PlaybackDspChain::last_was_bit_perfect()`; cleared when nothing decodes.
- GUI faceplate: `main_window::stream_label` ("44.1 kHz · 16-bit · ST · BIT-PERFECT" / "… · DSP · 48 kHz OUT") replaces the fixed "44.1kHz ST"; empty when nothing plays.
- Tests: `crates/amp-adapter-gui/tests/stream_info.rs` (44.1 kHz on 48 kHz output → resampled; 48 kHz at unity → bit-perfect; volume 0.85 → DSP; label format), TUI tests updated.
