---
id: TASK-070
title: Decompose amp-app/src/main.rs Monolith into Modular Subsystems
planId: PLAN-023
status: completed
role: architecture
verificationCommand: cargo check -p amp-app
---

# TASK-070: Decompose amp-app/src/main.rs Monolith into Modular Subsystems

## Description
Deconstruct the oversized monolithic file `crates/amp-app/src/main.rs` (477 lines) highlighted in `docs/orchestration-audit.md` section 5:
1. `cli.rs`: CLI argument parsing, subcommands (play, scan, daemon, tui, gui, info), and flags.
2. `runner.rs`: Execution orchestrator mapping CLI commands to core engine actions.
3. `terminal.rs`: Safe terminal raw-mode lifecycle guard, signal handler hooks, and cleanup on exit.
4. `composition.rs`: Pure-Rust dependency injection assembling driving and driven adapters into the runtime engine.
5. `main.rs`: Lightweight entry point (< 50 lines) initializing logger and delegating to `runner::run()`.

## Verification
```bash
cargo check -p amp-app
```

## Acceptance Criteria
- Every file in `crates/amp-app/src/` is strictly under 300 lines (target < 200 lines).
- Clean separation of CLI parsing, terminal handling, and engine composition.
- All existing CLI and TUI functionality preserved without regression.
