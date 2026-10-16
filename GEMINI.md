# Antigravity / Gemini Agent Guidelines

Follow the architecture defined in `docs/vision/` and tracked through Nexus Orchestrator in `.nexus/policy.json` and `.claude/`.

## Architecture & Verification
- Compile strictly using pure Rust crates.
- Run `./scripts/verify_pure_rust.sh` to ensure zero C build dependencies.
- Plan first in `.claude/plans/` and `.claude/tasks/`.
