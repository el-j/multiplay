# TASK-174: TUI as a Complete Player

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_tui`
- Priority: **P1**
- Audit findings: `UX-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-126, TASK-153

## Objectives
1. Add files/folders/URLs (path prompt with completion), seek (←/→), next/prev (b/z), EQ editing, volume/balance, repeat/shuffle, `?` help overlay, quit confirmation while playing.
2. Route all diagnostics to the log file (never stdout/stderr in raw mode).

## Acceptance Criteria (user-visible / measurable)
- Scripted TUI test (ratatui `TestBackend`) covers every keybinding.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-tui/src/`
- `crates/amp-app/src/`

## Verification Command
`cargo test -p amp-adapter-tui -p amp-app`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-047-ux)
- Keys (`crates/amp-app/src/tui_keys.rs`, one `KEY_HELP` table shown by `?`): x play, c/space pause-resume, v stop, z/b previous/next, ←/→ seek 5 s, +/- volume, Enter play selected, ↑/↓ select, Delete remove, o add file/folder/link (prompt with Tab completion, `complete_path`), j or / search, [ ] pick EQ band, { } band ∓1 dB (clamped ±12, sent to the engine), e/p windows, r repeat off/all/one, s shuffle, q quit (asks "Quit while playing? (y / n)" while playing; Ctrl+C always quits).
- Overlays (`tui_overlays.rs`): help, add prompt, quit question, status line ("Added 12 tracks", "skipped 1 that cannot play yet").
- Shared import: `amp_adapter_storage::import::import_paths` (files, folders, M3U/PLS/XSPF, CUE, skins, skipped-by-format) now used by the GUI and the TUI; `amp_domain::source_url::{looks_like_link, track_from_link}` shared for pasted links.
- Diagnostics go to the log file (PLAN-048); nothing prints to the terminal in raw mode.
- Tests: `crates/amp-app/tests/tui_keys.rs` (ratatui `TestBackend`: every help entry shown, prompt, Tab completion, transport with the real engine on the null sink, EQ, repeat/shuffle, quit confirmation), `amp-adapter-storage/tests/import.rs`, `amp-domain/tests/formats.rs` (links).
- Not done: balance (the engine has no balance control).
