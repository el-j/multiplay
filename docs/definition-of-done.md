# Evidence-Based Definition of Done (DoD v2)

## Philosophy & Core Rule

A task is **NEVER** completed simply because code compiles or a synthetic unit test passes against an in-memory mock.
**A task is completed ONLY when its intended user-visible or architectural capability is demonstrable on a real build and backed by verifiable evidence.**

---

## The 5 Verification Gates

Every task must pass all applicable gates before its status can transition to `Completed`:

### Gate 1: Pure-Rust Invariant & Soundness
- Zero bundled or vendored third-party C/C++ libraries (no OpenSSL, SQLite, FFmpeg, etc.).
- Audio decoding, key-value storage, and DSP must be pure Rust. TLS uses rustls; its `ring` crypto provider (C/asm) is the single documented exception in the allowlist.
- Any platform `-sys` bindings must be documented in `scripts/pure_rust_allowlist.txt`.
- Mandatory verification:
  ```bash
  ./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test
  ```

### Gate 2: Tests First & Real Adapter Verification
- Unit and integration tests must be written first (`testsFirst: true`).
- **Mocks Rule:** In-memory mocks or stubs may be used for unit testing isolated components, but **mocks can NEVER be used to mark a provider or network task Completed**.
- External services (YouTube, HTTP, Soulseek, etc.) must have contract tests using recorded golden fixtures or reproducible local test harness servers.
- Full workspace test pass:
  ```bash
  cargo test --workspace
  ```

### Gate 3: Hexagonal Modularity & Audio Thread Safety
- **Hexagonal Boundary:** Domain core (`amp-domain`) and engine (`amp-core-engine`) never import concrete adapter crates. Dependencies point strictly inwards via ports (`amp-ports`).
- **Real-Time Safety:** The CPAL real-time audio thread must be verified allocation-free, lock-free, and syscall-free. Communication with the audio thread occurs exclusively via lock-free ring buffers (`rtrb`).

### Gate 4: Code Quality & Line Limits
- `#![forbid(unsafe_code)]` in domain core and core engine.
- Every source file stays within `.nexus/policy.json` `maxFileLines` (500 lines).
- Zero compiler warnings, clippy warnings, or format errors:
  ```bash
  cargo fmt --check
  cargo clippy --workspace --all-targets -- -D warnings
  ```

### Gate 5: Manual Acceptance on Release Build
- Every task must document a manual verification step executed against a release or dev build:
  - Exact command run.
  - User-visible observation or telemetry recorded.
  - Link to logs, screenshot, or CI artifact.

---

## Task Template Standard

All new tasks must follow `.claude/tasks/_TEMPLATE.md` with explicit fields for:
1. `Plan Reference` & `Role`
2. `Audit findings` (findings from `AUDIT-2026-10-17` being resolved)
3. `Acceptance Criteria (user-visible / measurable)`
4. `Automated Verification Command`
5. `Manual Verification` (detailed evidence log)
