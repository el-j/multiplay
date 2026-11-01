# TASK-179: Measured Resource Budgets: Benchmark Harness & CI Tracking

## Plan Reference
- Plan: [PLAN-048](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-048-observability-and-diagnostics.md)
- Role: `qa_and_hardening`
- Priority: **P2**
- Audit findings: `QA-05`, `AUD-14`, `ORC-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Headless benchmark: engine + null sink playing FLAC/MP3/radio; record CPU% and RSS idle/active; GUI idle/playing measured manually per release.
2. Replace policy numbers with measured, per-front-end budgets; CI fails on > 20% regression.

## Acceptance Criteria (user-visible / measurable)
- Budgets in `.nexus/policy.json` match measurements in `docs/performance.md`.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `benches/`
- `docs/performance.md`
- `.nexus/policy.json`

## Verification Command
`cargo bench -p amp-core-engine`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31, branch feat/plan-048-observability)
- Harness: `crates/amp-adapter-gui/tests/resource_budget.rs` (ignored, release): real engine + real-time null sink, idle / FLAC / MP3 (20 s fixtures from `scripts/make_bench_fixtures.sh`), process CPU time and RSS from /proc, ps or Get-Process. CI step "Resource budgets" (Linux, macOS) writes the table to the run summary; fails when memory > 35 MB or CPU > measured baseline +20%.
- Profiling (macOS `sample`) found: sinc resampler ~60%, two FFTs per chunk ~30%, 2 ms wake-ups. Fixed: rubato `Fft` synchronous resampler (quality test `tests/resampler_quality.rs`: level ±0.1 dB, image < −100 dB, flat to 20 kHz), spectrum at most every 16 ms (`tests/spectrum_rate.rs`), 10 ms waits in null sink and full-ring worker path.
- Measured (M-series Mac, loaded): idle 0.75 → 0.17–0.33%, playing ~3% → 1.25–2.2%, RSS 8–11 MiB. Policy CPU targets (0.2% / 1.0%) not met yet; documented in `docs/performance.md`, policy text unchanged (guard test keeps policy, doc and test in step).
