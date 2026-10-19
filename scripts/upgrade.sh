#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo " Multiplay Toolchain & Dependency Upgrade"
echo "=========================================================="

# 1. Upgrade Rust compiler / toolchain to the latest version
echo "==> [1/4] Checking and upgrading Rust toolchain to latest..."
if command -v rustup >/dev/null 2>&1; then
    rustup update
    echo -e "\033[0;32m[SUCCESS]\033[0m Rust toolchain upgraded to latest:"
    rustc --version
    cargo --version
else
    echo -e "\033[0;33m[WARNING]\033[0m rustup not detected in PATH. Skipping rustup update."
fi

# 2. Upgrade crate dependencies, including new major versions (`cargo upgrade` is part of
#    cargo-edit). Breaking API changes then show up in the checks below and must be fixed.
echo "==> [2/4] Upgrading workspace dependencies (including major versions)..."
if ! cargo upgrade --version >/dev/null 2>&1; then
    echo -e "\033[0;31m[ERROR]\033[0m cargo-upgrade is missing. Install it with: cargo install cargo-edit --locked"
    exit 1
fi
cargo upgrade --incompatible allow

echo "Updating Cargo.lock to latest compatible versions..."
cargo update

# 3. Verify Pure-Rust compliance
echo "==> [3/4] Verifying 100% pure Rust compliance (Zero-C audit)..."
./scripts/verify_pure_rust.sh

# 4. Check workspace compilation and test suite
echo "==> [4/4] Validating workspace compilation with updated toolchain..."
cargo check --workspace --all-targets

echo -e "\033[0;32m[SUCCESS]\033[0m Multiplay toolchain and dependencies upgraded successfully!"
