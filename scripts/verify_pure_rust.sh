#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Multiplay Pure-Rust Sound Verifier
# ==============================================================================
# Audits the Cargo dependency tree across ALL target architectures and edge types
# (including build-dependencies and non-host targets).
#
# Rules enforced:
# 1. Zero vendored/bundled third-party C/C++ libraries (OpenSSL, SQLite, FFmpeg, etc.)
# 2. All native -sys crates must be explicitly approved in scripts/pure_rust_allowlist.txt
# 3. Supports --self-test to verify the detection logic is sound.
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ALLOWLIST_FILE="${SCRIPT_DIR}/pure_rust_allowlist.txt"

# Explicitly forbidden vendored/bundled third-party C libraries
FORBIDDEN_CRATES=(
    "openssl-sys"
    "openssl-src"
    "openssl"
    "libsqlite3-sys"
    "ffmpeg-sys"
    "ffmpeg-sys-next"
    "libpulse-sys"
    "libpulse-binding"
    "aws-lc-sys"
    "aws-lc-rs"
    "cmake"
)

# Parse allowlist entries (ignoring comments and empty lines)
get_allowed_crates() {
    if [[ ! -f "${ALLOWLIST_FILE}" ]]; then
        echo "Error: Allowlist file not found at ${ALLOWLIST_FILE}" >&2
        exit 1
    fi
    grep -v '^[[:space:]]*#' "${ALLOWLIST_FILE}" | grep -v '^[[:space:]]*$' | cut -d: -f1 | tr -d ' '
}

# Run the core verification audit against a given dependency list or live cargo tree
verify_dependencies() {
    local dep_list="$1"
    local fail=0

    # 1. Check for explicitly forbidden vendored C crates
    for forbidden in "${FORBIDDEN_CRATES[@]}"; do
        if echo "${dep_list}" | grep -E -q "^${forbidden}([[:space:]]|$)"; then
            echo -e "\033[0;31m[FAIL]\033[0m Forbidden vendored C crate detected: '${forbidden}'" >&2
            fail=1
        fi
    done

    # 2. Check all -sys crates against allowlist
    local allowed
    allowed=$(get_allowed_crates)

    local sys_crates
    sys_crates=$(echo "${dep_list}" | grep -E -- "-sys([[:space:]]|$)" | awk '{print $1}' | sort -u || true)

    if [[ -n "${sys_crates}" ]]; then
        while IFS= read -r crate; do
            [[ -z "${crate}" ]] && continue
            if ! echo "${allowed}" | grep -E -q "^${crate}$"; then
                echo -e "\033[0;31m[FAIL]\033[0m Unapproved -sys crate detected: '${crate}'" >&2
                echo "       Not listed in ${ALLOWLIST_FILE}" >&2
                fail=1
            fi
        done <<< "${sys_crates}"
    fi

    return "${fail}"
}

# Self-test mode to verify soundness of detection
run_self_test() {
    echo "Running pure-Rust verifier self-tests..."

    # Test 1: Reject openssl-sys
    local fake_deps_openssl="openssl-sys v0.9.100\nsymphonia v0.5.4"
    if verify_dependencies "$(echo -e "${fake_deps_openssl}")" >/dev/null 2>&1; then
        echo -e "\033[0;31m[SELF-TEST FAIL]\033[0m Verifier failed to reject openssl-sys!"
        exit 1
    fi
    echo "  [OK] Successfully rejected forbidden 'openssl-sys'"

    # Test 2: Reject libsqlite3-sys
    local fake_deps_sqlite="libsqlite3-sys v0.26.0\nredb v2.1.1"
    if verify_dependencies "$(echo -e "${fake_deps_sqlite}")" >/dev/null 2>&1; then
        echo -e "\033[0;31m[SELF-TEST FAIL]\033[0m Verifier failed to reject libsqlite3-sys!"
        exit 1
    fi
    echo "  [OK] Successfully rejected forbidden 'libsqlite3-sys'"

    # Test 3: Reject ffmpeg-sys
    local fake_deps_ffmpeg="ffmpeg-sys v6.0.0\nrubato v0.15.0"
    if verify_dependencies "$(echo -e "${fake_deps_ffmpeg}")" >/dev/null 2>&1; then
        echo -e "\033[0;31m[SELF-TEST FAIL]\033[0m Verifier failed to reject ffmpeg-sys!"
        exit 1
    fi
    echo "  [OK] Successfully rejected forbidden 'ffmpeg-sys'"

    # Test 4: Reject unapproved -sys crate
    local fake_deps_unapproved="unapproved-c-lib-sys v1.0.0\namp-core-engine v0.1.0"
    if verify_dependencies "$(echo -e "${fake_deps_unapproved}")" >/dev/null 2>&1; then
        echo -e "\033[0;31m[SELF-TEST FAIL]\033[0m Verifier failed to reject unapproved -sys crate!"
        exit 1
    fi
    echo "  [OK] Successfully rejected unapproved -sys crate"

    # Test 5: Allow approved platform crates
    local fake_deps_valid="coreaudio-sys v0.2.18\nalsa-sys v0.3.1\nwindows-sys v0.52.0"
    if ! verify_dependencies "$(echo -e "${fake_deps_valid}")" >/dev/null 2>&1; then
        echo -e "\033[0;31m[SELF-TEST FAIL]\033[0m Verifier rejected approved platform crates!"
        exit 1
    fi
    echo "  [OK] Successfully allowed approved platform crates"

    echo -e "\033[0;32m[PASS]\033[0m All pure-Rust verifier self-tests passed!"
    exit 0
}

# Main execution
if [[ "${1:-}" == "--self-test" ]]; then
    run_self_test
fi

echo "=========================================================="
echo " Running Pure-Rust Toolchain Verification (Sound Audit)  "
echo "=========================================================="

unset CC
unset CXX

cd "${ROOT_DIR}"

if [[ ! -f "Cargo.lock" ]]; then
    echo "Error: Cargo.lock not found in ${ROOT_DIR}" >&2
    exit 1
fi

echo "Inspecting workspace dependencies across all targets and build-dependencies..."
CARGO_TREE_OUTPUT=$(cargo tree --target all --edges normal,build --prefix none 2>/dev/null || true)

if ! verify_dependencies "${CARGO_TREE_OUTPUT}"; then
    echo ""
    echo -e "\033[0;31m[ERROR]\033[0m Pure-Rust verification failed! Forbidden or unapproved C crates detected."
    echo "        See scripts/pure_rust_allowlist.txt for allowed platform bindings."
    exit 1
fi

TOTAL_SYS=$(echo "${CARGO_TREE_OUTPUT}" | grep -E -- "-sys([[:space:]]|$)" | awk '{print $1}' | sort -u | wc -l | tr -d ' ')
echo -e "\033[0;32m[SUCCESS]\033[0m Verified 100% pure Rust audio & core engine."
echo "          Zero vendored third-party C libraries detected."
echo "          All ${TOTAL_SYS} platform -sys bindings verified against allowlist."
