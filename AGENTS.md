# Agent Working Rules for multiplay

## Non-Negotiables
1. **Pure Rust (`#![forbid(unsafe_code)]`):** Zero C/C++ dynamic or static linkage. No OpenSSL, SQLite, FFmpeg, libpulse, or libasound.
2. **Hexagonal Modularity:** Domain Core (`amp-domain`) and Core Engine (`amp-core-engine`) must never import concrete adapters. All boundary crossings go through Ports (`amp-ports`).
3. **Audio Thread Real-Time Safety:** The CPAL audio callback must NEVER allocate, wait on locks, make system calls, or access the file system.
4. **Nexus Orchestrator Tracking:**
   - Check `.claude/orchestrator-index.md` and `.nexus/policy.json` before starting work.
   - Every change must map to a task in `.claude/tasks/TASK-NNN-*.md`.
   - Run verification commands prior to marking any task done.
5. **Quality Gates:** 100% pure Rust verified with `./scripts/verify_pure_rust.sh`. Tests must pass without warnings or errors.
