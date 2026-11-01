# TASK-186: Soak, Stress & Failure-Injection Tests

## Plan Reference
- Plan: [PLAN-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-050-release-grade-quality-assurance.md)
- Role: `qa_and_hardening`
- Priority: **P2**
- Audit findings: `QA-08`, `AUD-10` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-136, TASK-133

## Objectives
1. 8 h headless soak (memory growth < 5 MB/h), 1,000 rapid skips, network drop/slow server (1 KB/s), resolver timeouts, disk-full during download, device loss.

## Acceptance Criteria (user-visible / measurable)
- Weekly scheduled workflow green; results in `docs/performance.md`.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/tests/soak.rs`
- `.github/workflows/soak.yml`

## Verification Command
`cargo test --release -p amp-core-engine --test soak -- --ignored`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-050-qa)
- `crates/amp-adapter-gui/tests/stress.rs` (every CI run): 1,000 rapid skips (no leaked threads, RSS back), 1 KB/s trickling server (Buffering shown, Stop < 1 s), server hang-up mid-stream (ends/fails, no hang), never-answering resolver (error after the timeout, next track plays). Opt-in soak (`SOAK_MINUTES`) fits RSS growth by least squares, fails at ≥ 5 MiB/h; `.github/workflows/soak.yml` weekly, 120 min.
- Bug found and fixed: a slow network stream stuttered while reporting "Playing". New `watch_stream_buffer`: for network sources, when the output ring runs nearly dry the output is held (5 ms fade) and the state is Buffering; it resumes at 3/4 of the ring. Local files are unaffected (gapless and DJ tests pass, dj_toggle 5/5).
- Open: disk-full during download and device-loss injection (no injectable seam yet).
