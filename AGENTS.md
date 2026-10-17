# Agent Working Rules for multiplay

## Non-Negotiables
1. **Pure Rust (`#![forbid(unsafe_code)]` in domain & core engine):** Zero bundled or vendored third-party C/C++ libraries (no OpenSSL, SQLite, FFmpeg, aws-lc-sys, or cmake builds). Audio decoding, storage, TLS, and DSP are pure Rust. Thin host OS platform `-sys` bindings are governed strictly by `scripts/pure_rust_allowlist.txt`.
2. **Hexagonal Modularity:** Domain Core (`amp-domain`) and Core Engine (`amp-core-engine`) must never import concrete adapters. All boundary crossings go through Ports (`amp-ports`).
3. **Audio Thread Real-Time Safety:** The CPAL audio callback must NEVER allocate, wait on locks, make system calls, or access the file system.
4. **Nexus Orchestrator Tracking:**
   - Check `.claude/orchestrator-index.md` and `.nexus/policy.json` before starting work.
   - Every change must map to a task in `.claude/tasks/TASK-NNN-*.md`.
   - Run verification commands prior to marking any task done.
5. **Quality Gates:** 100% pure Rust verified with `./scripts/verify_pure_rust.sh`. Tests must pass without warnings or errors.
