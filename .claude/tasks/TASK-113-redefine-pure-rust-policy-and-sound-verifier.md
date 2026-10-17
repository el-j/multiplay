# TASK-113: Redefine the Pure-Rust Policy & Rewrite a Sound Verifier

## Plan Reference
- Plan: [PLAN-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-034-release-truth-reset-and-governance.md)
- Role: `architecture`
- Priority: **P1**
- Audit findings: `ORC-05`, `BLD-01` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Rewrite the policy in `CLAUDE.md`, `AGENTS.md`, `GEMINI.md` and `.nexus/policy.json`: no bundled/vendored third-party C libraries; thin `-sys` bindings to OS platform APIs are allowed via explicit allowlist.
2. Rewrite `scripts/verify_pure_rust.sh` to inspect `--target all -e normal,build` and fail on any `-sys`/C-building crate not in `scripts/pure_rust_allowlist.txt` (initial allowlist: alsa-sys, coreaudio-sys, objc-sys, ring, libloading, windows-sys …, each with a justification).
3. Fail on known vendoring crates (openssl-src, openssl-sys, libsqlite3-sys, ffmpeg-sys*, aws-lc-sys, cmake).

## Acceptance Criteria (user-visible / measurable)
- Verifier passes on macOS, Linux and Windows CI with the allowlist.
- Adding `openssl-sys` to any crate makes the verifier fail (tested in a script self-test).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `scripts/verify_pure_rust.sh`
- `scripts/pure_rust_allowlist.txt`
- `CLAUDE.md`
- `.nexus/policy.json`

## Verification Command
`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`

## Manual Verification
- `./scripts/verify_pure_rust.sh` and `./scripts/verify_pure_rust.sh --self-test` executed and verified green.
- `scripts/pure_rust_allowlist.txt` created with justifications for all platform `-sys` bindings.
- Sound check inspects `--target all --edges normal,build` without false negatives or hidden build-dependencies.
- Self-test confirmed rejection of `openssl-sys`, `libsqlite3-sys`, `ffmpeg-sys`, and unapproved `-sys` crates.
- Policy updated across `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, and `.nexus/policy.json`.

## Status
- **Status:** Completed
