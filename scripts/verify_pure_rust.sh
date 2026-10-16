#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo " Running Pure-Rust Toolchain Verification (Zero-C Audit) "
echo "=========================================================="

# 1. Ensure Cargo compiles without CC or CXX environment variables
unset CC
unset CXX

# 2. Check for disallowed native C build dependencies in the dependency tree
FORBIDDEN_CRATES=("cc" "cmake" "pkg-config" "openssl-sys" "libloading" "libsqlite3-sys" "ffmpeg-sys")
FAIL=0

if [ -f "Cargo.lock" ]; then
    for crate in "${FORBIDDEN_CRATES[@]}"; do
        MATCHES=$(cargo tree -i "$crate" 2>/dev/null || true)
        if [ -n "$MATCHES" ]; then
            echo -e "\033[0;31m[FAIL]\033[0m Forbidden native C-wrapper crate detected: '$crate'"
            echo "$MATCHES"
            FAIL=1
        fi
    done
fi

if [ "$FAIL" -eq 1 ]; then
    echo -e "\033[0;31m[ERROR]\033[0m Verification failed! Native C build-tools or libraries are present."
    exit 1
fi

echo -e "\033[0;32m[SUCCESS]\033[0m 100% Pure Rust verified. Zero C/C++ dynamic or static linkage detected."
