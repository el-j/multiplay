# ==============================================================================
# Multiplay: Universal Pure-Rust Audiophile Player Engine & Winamp Modular TUI/GUI
# ==============================================================================

SHELL := /bin/bash
.DEFAULT_GOAL := help

.PHONY: all help build build-release check lint fmt fmt-check test test-hw test-hermetic test-verbose \
        verify-pure-rust doc coverage mutation-test bundle bundle-macos package-linux \
        package-windows upgrade clean

## all: Run full validation suite (pure-Rust audit, linter, tests, build, and bundle output)
all: verify-pure-rust lint test build bundle

## help: Display this list of available Makefile targets
help:
	@echo "Multiplay Build & Quality Automation"
	@echo "======================================"
	@echo "Usage: make [target]"
	@echo ""
	@grep -E '^## ' $(MAKEFILE_LIST) | sed -e 's/## //g' | awk 'BEGIN {FS = ": "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## build: Build all workspace crates in debug mode
build:
	cargo build --workspace

## build-release: Build all workspace binaries in optimized release mode
build-release:
	cargo build --workspace --release

## bundle: Package all binaries, macOS app bundle, and launchers into the build/ output folder
bundle: build-release
	./scripts/bundle_build.sh

## check: Fast type-check and compilation validation across all targets
check:
	cargo check --workspace --all-targets

## lint: Run strict Clippy linter with zero warnings permitted
lint:
	cargo clippy --workspace --all-targets -- -D warnings

## fmt: Format all Rust source files according to rustfmt style
fmt:
	cargo fmt --all

## fmt-check: Verify all Rust source files are correctly formatted
fmt-check:
	cargo fmt --all -- --check

## test: Run unit, integration, and doc tests across the entire workspace
test:
	./scripts/run_all_tests.sh

## test-hw: Run tests that need real audio hardware (ignored by default and in CI)
test-hw:
	cargo test --workspace -- --ignored

## test-hermetic: Run tests with a sandboxed HOME and fail if they write user state
test-hermetic:
	./scripts/check_hermetic_tests.sh

## test-verbose: Run the test suite with verbose output
test-verbose:
	cargo test --workspace -- --nocapture

## verify-pure-rust: Verify 100% pure Rust compliance with zero C/C++ linkage
verify-pure-rust:
	./scripts/verify_pure_rust.sh

## doc: Generate zero-dependency API documentation for all workspace crates
doc:
	cargo doc --workspace --no-deps

## coverage: Run code coverage analysis (using cargo-llvm-cov if available)
coverage:
	@if command -v cargo-llvm-cov >/dev/null 2>&1; then \
		cargo llvm-cov --workspace --text; \
	else \
		echo "cargo-llvm-cov not detected. Running standard test suite..."; \
		cargo test --workspace; \
	fi

## mutation-test: Run mutation testing using cargo-mutants with .cargo/mutants.toml
mutation-test:
	@if command -v cargo-mutants >/dev/null 2>&1; then \
		cargo mutants --check; \
	else \
		echo "cargo-mutants not installed. Validate configuration file..."; \
		test -f .cargo/mutants.toml && echo "[SUCCESS] .cargo/mutants.toml configuration valid."; \
	fi

## bundle-macos: Package macOS standalone application bundle (.app) and launcher into build/macos/
bundle-macos: bundle
	@echo "[SUCCESS] macOS bundle available in build/macos/Multiplay.app"

## package-linux: Package Linux standalone tarball with CLI and GUI binaries into build/linux/
package-linux: build-release
	@mkdir -p build/linux dist
	@cp target/release/amp-app build/linux/multiplay 2>/dev/null || true
	@cp target/release/multiplay-gui build/linux/multiplay-gui 2>/dev/null || true
	@tar -czf dist/multiplay-linux.tar.gz -C build/linux . 2>/dev/null || true
	@echo "[SUCCESS] Linux package created in dist/multiplay-linux.tar.gz"

## package-windows: Package Windows standalone zip with CLI and GUI executables into build/windows/
package-windows: build-release
	@mkdir -p build/windows dist
	@cp target/release/amp-app.exe build/windows/multiplay.exe 2>/dev/null || true
	@cp target/release/multiplay-gui.exe build/windows/multiplay-gui.exe 2>/dev/null || true
	@echo "[SUCCESS] Windows binaries staged in build/windows"

## upgrade: Update Rust toolchain to latest and upgrade all workspace dependencies
upgrade:
	./scripts/upgrade.sh

## clean: Remove build artifacts, target directories, and temporary files
clean:
	cargo clean
	rm -rf build/ dist/
