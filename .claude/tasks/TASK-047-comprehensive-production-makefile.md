---
id: TASK-047
title: Comprehensive Production Makefile
planId: PLAN-015
status: completed
role: devops_and_build
verificationCommand: make help && make verify-pure-rust && make check
---

# TASK-047: Comprehensive Production Makefile

## Description
Create a production-grade `Makefile` in the repository root standardizing all developer, QA, and CI operations:
1. Targets:
   - `build` / `build-release`: Builds all workspace crates in debug/release.
   - `test`: Runs unit, integration, and doc tests across the entire workspace.
   - `verify-pure-rust`: Runs `./scripts/verify_pure_rust.sh` ensuring zero C/C++ dynamic or static linkage.
   - `lint`: Runs `cargo clippy --workspace --all-targets -- -D warnings`.
   - `check`: Runs compilation checks across all targets.
   - `coverage`: Generates code coverage report via `cargo llvm-cov` or `cargo tarpaulin`.
   - `mutation-test`: Runs mutation testing using `cargo-mutants` configured via `.cargo/mutants.toml`.
   - `doc`: Generates zero-dependency rustdoc documentation via `cargo doc --workspace --no-deps`.
   - `bundle-macos`: Packages macOS `Multiplay.app` bundle and standalone launcher scripts.
   - `clean`: Cleans build targets and temporary files.
   - `help`: Displays documented target descriptions.

## Verification
```bash
make help && make verify-pure-rust && make check
```
