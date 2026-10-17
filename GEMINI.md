# Antigravity / Gemini Agent Guidelines

Follow the architecture defined in `docs/vision/` and tracked through Nexus Orchestrator in `.nexus/policy.json` and `.claude/`.

## Architecture & Verification
- Compile strictly using pure Rust crates (no vendored C libraries).
- Run `./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test` to enforce allowlisted OS platform bindings.
- Plan first in `.claude/plans/` and `.claude/tasks/`.
