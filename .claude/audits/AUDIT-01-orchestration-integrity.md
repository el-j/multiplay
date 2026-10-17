# AUDIT-01 — Orchestration Integrity

Part of [AUDIT-2026-10-17](README.md).

The orchestration system is supposed to be the source of truth for what is done. Today it records *intent*, not *verified outcome*. This is the root cause behind most other findings: work is marked complete once a unit test against a mock passes, not when a user-visible capability works.

## ORC-01 — Task status is inflated relative to shipped behaviour · P0

**Evidence**
- `.claude/plans/PLAN-008-*` marks TASK-023 (Soulseek protocol adapter) and TASK-024 (libp2p mesh) *Completed*. `crates/amp-adapter-soulseek/src/lib.rs:43` and `crates/amp-adapter-p2p/src/lib.rs` are in-memory `Mutex<Vec<…>>`/`HashMap` catalogs. Neither crate depends on a socket, `libp2p` or `tokio::net`.
- TASK-015 claims a "`librespot-core` Spotify adapter". `crates/amp-adapter-spotify/Cargo.toml` has no librespot dependency, and `authenticate()` accepts any non-empty string (`crates/amp-adapter-spotify/src/lib.rs:31`).
- PLAN-021 ("Elimination of synthetic stubs") swapped fake sine-wave audio for `Err(NotFound)` caches. That removes the *lie in the audio* but not the *lie in the status*.
- TASK-019 (search modal) is *Completed*, but no UI ever executes a search (see UX-01).
- TASK-109 claims EBU R128 normalization. No loudness is measured anywhere, and R128 tags are misparsed (DSP-05).
- TASK-044 claims a "secure keyring adapter". `crates/amp-adapter-storage/src/keyring.rs:63` is an in-process `RwLock<HashMap>` that is lost on exit.

**Impact:** Planning decisions are made against a false baseline. New features keep stacking (DJ engine, mobile jam) on top of unfinished foundations.

**Recommendation:** Re-open every task whose user-visible outcome is not demonstrable, through the Nexus tools (policy forbids editing task state by hand). Adopt an evidence-based Definition of Done (ORC-04). → TASK-112

## ORC-02 — Three different "metrics" sources disagree · P2

**Evidence:** The `orchestrator-index.md` header says "29 Plans | 94 Tasks | 90 Completed". `orchestrator.json` metrics say 30/98 with 94 completed, `activePlan: PLAN-030`. The files on disk hold 33 plans and 111 tasks, and PLAN-031 … 033 are marked Completed in the plan files.

**Recommendation:** Generate the metrics from plan/task files (`discover: true` is already in `.nexus/policy.json`). Never hand-maintain counts. → TASK-112

## ORC-03 — Verification commands don't verify the claim · P1

**Evidence:** Task verification commands are almost always `cargo test -p <crate>`. Those pass against mocks; for example, TASK-023's tests register fake peer files and then "find" them. Several task verification commands reference `make all`, which includes `bundle` (a macOS-only script) and `lint`. Because CI runs `fmt --check`, which fails, `make all` is not what CI enforces.

**Recommendation:** Every task must have **(a)** an automated check that exercises the real adapter (recorded fixture or local test server), and **(b)** a manual acceptance step on a real build ("search 'boards of canada' on Soulseek, play a result"). → TASK-115

## ORC-04 — The Definition of Done is aspirational and unmeasurable · P2

**Evidence:** `docs/orchestration-audit.md` (2026-10-16) lists "100% mutation tested" and "100% coverage" as criteria. A day later the same pattern of inflated completion recurred. The policy also cites resource budgets (`idle CPU < 0.2%`, `≤ 35 MB`) that are never measured anywhere in the repo.

**Recommendation:** Replace it with a measurable DoD: CI green on 3 OSes, a feature demo checklist, measured budgets with a benchmark job, and a mutation-score threshold on the core crates only. → TASK-115, TASK-179

## ORC-05 — The "pure Rust" policy is unattainable as written and silently violated · P1

**Evidence:** `CLAUDE.md` and `.nexus/policy.json` forbid any C linkage, including "libasound". On Linux `cpal 0.15` → `alsa` → `alsa-sys` links `libasound` (`cargo tree --target all -i alsa-sys`). On macOS `coreaudio-sys` uses bindgen at build time. `ring` (through rustls) compiles C and assembly. The verifier hides all of this (BLD-01).

**Recommendation:** Restate the policy precisely. "No *bundled or vendored* third-party C libraries (OpenSSL, SQLite, FFmpeg). Thin `-sys` bindings to OS-provided platform APIs (CoreAudio, ALSA/PipeWire, WASAPI, Metal/Vulkan) are allowed and allow-listed." Make the verifier enforce *that*, on all targets. → TASK-113

## ORC-06 — Feature breadth outpaced foundations · P1 (process)

**Evidence:** PLAN-031 (pro DJ engine), PLAN-032 (mobile jam), PLAN-012 (CRDT) and PLAN-013 (multi-deck) were all built while core transport, search wiring, persistence and the headline providers were still missing.

**Recommendation:** Freeze new feature plans until milestones M0 to M2 are done (see the [plan map](README.md#finding--plan-map)). Mark the DJ and jam features **Experimental** and hide them behind a setting. → TASK-112, TASK-176
