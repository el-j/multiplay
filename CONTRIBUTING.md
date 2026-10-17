# Contributing to multiplay

Thank you for your interest in contributing to **multiplay**! We are committed to building a reliable, transparent, audiophile-grade music player written in pure Rust.

---

## Non-Negotiable Core Rules

All contributions must strictly satisfy the following technical invariants:

1. **Pure Rust (`#![forbid(unsafe_code)]`):**
   - Core domain (`amp-domain`) and engine (`amp-core-engine`) must have `#![forbid(unsafe_code)]`.
   - Zero vendored or bundled third-party C/C++ libraries (no OpenSSL, SQLite, FFmpeg, etc.).
   - Any new platform `-sys` binding must be an OS-level API and must be approved in `scripts/pure_rust_allowlist.txt`.
   - Verification command `./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test` must pass.

2. **Hexagonal Architecture:**
   - `amp-domain` contains domain logic and entities; depends on nothing.
   - `amp-ports` defines driving and driven interfaces; depends only on `amp-domain`.
   - `amp-core-engine` implements domain services and ports; never imports adapter crates.
   - Adapters (`amp-adapter-*`) implement ports; never import other adapters directly.

3. **Audio Thread Real-Time Safety:**
   - The CPAL audio callback thread must NEVER allocate memory, acquire blocking locks, make system calls, or access the file system. Use lock-free SPSC ring buffers (`rtrb`).

4. **File Length Limit:**
   - Every source file must be $\le 300$ lines. Decompose large files into focused submodules.

5. **Definition of Done:**
   - All tasks must adhere to `docs/definition-of-done.md`. Mocks may satisfy unit tests, but tasks representing user-visible features cannot be marked complete without end-to-end demonstrable behavior.

---

## Development Workflow

1. **Fork and Clone:**
   ```bash
   git clone https://github.com/<your-username>/multiplay.git
   cd multiplay
   ```

2. **Branching:**
   - Create a feature branch: `git checkout -b feat/task-nnn-description`

3. **Writing Tests First:**
   - Write failing unit/integration tests that describe the desired functionality before writing implementation code.

4. **Running Quality Gates:**
   ```bash
   # 1. Pure Rust verification
   ./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test

   # 2. Workspace unit & integration tests
   cargo test --workspace

   # 3. Formatting and lints
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   ```

5. **Submitting a Pull Request:**
   - Open a PR against `main`. Ensure CI checks pass on macOS, Linux, and Windows.
