#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Multiplay Pure-Rust Verifier
# ==============================================================================
# Audits the resolved Cargo dependency graph across ALL targets, including
# build-dependencies, and fails closed if the graph cannot be inspected.
#
# Rules enforced (see scripts/pure_rust_allowlist.txt for the policy text):
# 1. Known vendoring / bundled C library crates are forbidden outright.
# 2. Every `-sys` crate (native link) must be allowlisted.
# 3. Every crate that compiles or locates native code at build time — i.e. a
#    direct dependent of a native build tool (cc, cmake, pkg-config, bindgen,
#    vcpkg, cxx-build, nasm-rs) — must be allowlisted.
#
# Usage:
#   ./scripts/verify_pure_rust.sh              # audit this workspace
#   ./scripts/verify_pure_rust.sh --self-test  # prove the detection logic works
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ALLOWLIST_FILE="${ALLOWLIST_FILE:-${SCRIPT_DIR}/pure_rust_allowlist.txt}"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[0;33m'
RESET=$'\033[0m'

# Crates that vendor or build third-party C/C++ libraries. Never allowed.
FORBIDDEN_CRATES=(
    "openssl-sys" "openssl-src" "openssl"
    "libsqlite3-sys"
    "ffmpeg-sys" "ffmpeg-sys-next" "ffmpeg-next"
    "libpulse-sys" "libpulse-binding"
    "aws-lc-sys" "aws-lc-rs" "aws-lc-fips-sys"
    "cmake"
)

# Build tools whose direct dependents compile, link or locate native code.
NATIVE_BUILD_TOOLS=("cc" "cmake" "pkg-config" "bindgen" "vcpkg" "cxx-build" "nasm-rs")

fail() { echo "${RED}[FAIL]${RESET} $*" >&2; }

# Prints allowlisted crate names, one per line.
allowed_crates() {
    if [[ ! -f "${ALLOWLIST_FILE}" ]]; then
        fail "Allowlist not found at ${ALLOWLIST_FILE}"
        return 1
    fi
    grep -v '^[[:space:]]*#' "${ALLOWLIST_FILE}" | grep -v '^[[:space:]]*$' | cut -d: -f1 | tr -d ' '
}

# Runs `cargo tree` for the workspace in $1 with extra args; fails closed.
# A missing package for `-i <name>` is reported as empty output, not an error.
cargo_tree() {
    local dir="$1"; shift
    local out err status=0
    err="$(mktemp)"
    out="$(cd "${dir}" && cargo tree ${CARGO_TREE_FLAGS:-} --target all -e normal,build --prefix none "$@" 2>"${err}")" || status=$?
    if [[ ${status} -ne 0 ]]; then
        if grep -q "did not match any packages" "${err}"; then
            rm -f "${err}"
            return 0
        fi
        fail "cargo tree failed in ${dir} (exit ${status}):"
        cat "${err}" >&2
        rm -f "${err}"
        return 2
    fi
    rm -f "${err}"
    printf '%s\n' "${out}"
}

# Extracts unique crate names from `cargo tree --prefix none` output on stdin.
crate_names() { awk 'NF {print $1}' | sort -u; }

# Audits the workspace in $1. Returns 0 when compliant, 1 on violations,
# 2 when the dependency graph could not be inspected.
audit_workspace() {
    local dir="$1" violations=0 tree allowed
    allowed="$(allowed_crates)" || return 2
    tree="$(cargo_tree "${dir}")" || return 2
    if [[ -z "${tree}" ]]; then
        fail "cargo tree produced no output for ${dir}"
        return 2
    fi
    local names
    names="$(printf '%s\n' "${tree}" | crate_names)"

    local crate
    for crate in "${FORBIDDEN_CRATES[@]}"; do
        if grep -qxF "${crate}" <<<"${names}"; then
            fail "Forbidden vendored C crate in dependency graph: '${crate}'"
            violations=1
        fi
    done

    # Candidates: every -sys crate plus every direct dependent of a build tool.
    local candidates
    candidates="$(grep -E -- '-sys$' <<<"${names}" || true)"
    local tool dependents
    for tool in "${NATIVE_BUILD_TOOLS[@]}"; do
        grep -qxF "${tool}" <<<"${names}" || continue
        dependents="$(cargo_tree "${dir}" -i "${tool}" --depth 1)" || return 2
        dependents="$(printf '%s\n' "${dependents}" | crate_names | grep -vxF "${tool}" || true)"
        candidates="$(printf '%s\n%s\n' "${candidates}" "${dependents}")"
    done

    while IFS= read -r crate; do
        [[ -z "${crate}" ]] && continue
        if ! grep -qxF "${crate}" <<<"${allowed}"; then
            fail "Native-code crate not in allowlist: '${crate}'"
            violations=1
        fi
    done < <(printf '%s\n' "${candidates}" | sort -u)

    # Stale allowlist entries hide policy drift; report them without failing.
    while IFS= read -r crate; do
        [[ -z "${crate}" ]] && continue
        if ! grep -qxF "${crate}" <<<"${names}"; then
            echo "${YELLOW}[WARN]${RESET} Allowlist entry not present in dependency graph: '${crate}'" >&2
        fi
    done <<<"${allowed}"

    if [[ ${violations} -eq 0 ]]; then
        CHECKED_COUNT="$(printf '%s\n' "${candidates}" | grep -c . || true)"
    fi
    return "${violations}"
}

# ---------------------------------------------------------------------------
# Self-test: builds throwaway offline workspaces with path-only fake crates and
# asserts the live audit rejects each violation class and accepts a clean graph.
# ---------------------------------------------------------------------------
make_crate() {
    # make_crate <dir> <name> [<dep-section> <dep-name>]...
    local dir="$1" name="$2"; shift 2
    mkdir -p "${dir}/${name}/src"
    : >"${dir}/${name}/src/lib.rs"
    {
        printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2021"\n' "${name}"
        while [[ $# -ge 2 ]]; do
            printf '\n[%s]\n%s = { path = "../%s" }\n' "$1" "$2" "$2"
            shift 2
        done
    } >"${dir}/${name}/Cargo.toml"
}

make_workspace() {
    # make_workspace <dir> <member>...
    local dir="$1"; shift
    local members=""
    local m
    for m in "$@"; do members+="\"${m}\", "; done
    printf '[workspace]\nresolver = "2"\nmembers = [%s]\n' "${members}" >"${dir}/Cargo.toml"
}

expect_status() {
    # expect_status <label> <expected: pass|fail> <workspace-dir>
    local label="$1" expected="$2" dir="$3" status=0
    CARGO_TREE_FLAGS="--offline" ALLOWLIST_FILE="${SELFTEST_ALLOWLIST}" \
        audit_workspace "${dir}" >/dev/null 2>&1 || status=$?
    if [[ "${expected}" == "pass" && ${status} -eq 0 ]] || [[ "${expected}" == "fail" && ${status} -ne 0 ]]; then
        echo "  [OK] ${label}"
    else
        echo "${RED}[SELF-TEST FAIL]${RESET} ${label} (exit ${status}, expected ${expected})"
        SELFTEST_FAILED=1
    fi
}

run_self_test() {
    echo "Running pure-Rust verifier self-tests..."
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "${tmp}"' EXIT
    SELFTEST_FAILED=0
    SELFTEST_ALLOWLIST="${tmp}/allowlist.txt"
    printf '# test allowlist\nplatform-sys: approved OS binding\ngood-native: approved C shim\n' >"${SELFTEST_ALLOWLIST}"

    local w
    w="${tmp}/clean";       mkdir -p "$w"; make_crate "$w" app dependencies platform-sys; make_crate "$w" platform-sys; make_workspace "$w" app
    expect_status "accepts allowlisted -sys crate" pass "$w"

    w="${tmp}/openssl";     mkdir -p "$w"; make_crate "$w" app dependencies openssl-sys; make_crate "$w" openssl-sys; make_workspace "$w" app
    expect_status "rejects openssl-sys" fail "$w"

    w="${tmp}/sqlite";      mkdir -p "$w"; make_crate "$w" app dependencies libsqlite3-sys; make_crate "$w" libsqlite3-sys; make_workspace "$w" app
    expect_status "rejects libsqlite3-sys" fail "$w"

    w="${tmp}/unapproved";  mkdir -p "$w"; make_crate "$w" app dependencies mystery-sys; make_crate "$w" mystery-sys; make_workspace "$w" app
    expect_status "rejects unapproved -sys crate" fail "$w"

    w="${tmp}/cc-dep";      mkdir -p "$w"; make_crate "$w" app dependencies sneaky; make_crate "$w" sneaky build-dependencies cc; make_crate "$w" cc; make_workspace "$w" app
    expect_status "rejects non -sys crate compiling C via cc build-dependency" fail "$w"

    w="${tmp}/cc-ok";       mkdir -p "$w"; make_crate "$w" app dependencies good-native; make_crate "$w" good-native build-dependencies cc; make_crate "$w" cc; make_workspace "$w" app
    expect_status "accepts allowlisted crate using cc" pass "$w"

    w="${tmp}/target-dep";  mkdir -p "$w"; make_crate "$w" app "target.'cfg(target_os = \"linux\")'.dependencies" hidden-sys; make_crate "$w" hidden-sys; make_workspace "$w" app
    expect_status "rejects -sys crate behind a non-host target cfg" fail "$w"

    w="${tmp}/broken";      mkdir -p "$w"; printf 'this is not toml' >"$w/Cargo.toml"
    expect_status "fails closed when cargo tree cannot run" fail "$w"

    if [[ ${SELFTEST_FAILED} -ne 0 ]]; then
        echo "${RED}[FAIL]${RESET} Pure-Rust verifier self-tests failed."
        exit 1
    fi
    echo "${GREEN}[PASS]${RESET} All pure-Rust verifier self-tests passed."
    exit 0
}

if [[ "${1:-}" == "--self-test" ]]; then
    run_self_test
fi

echo "=========================================================="
echo " Pure-Rust Dependency Verification (all targets + build) "
echo "=========================================================="

if [[ ! -f "${ROOT_DIR}/Cargo.lock" ]]; then
    fail "Cargo.lock not found in ${ROOT_DIR}"
    exit 1
fi

CHECKED_COUNT=0
status=0
CARGO_TREE_FLAGS="--locked" audit_workspace "${ROOT_DIR}" || status=$?
if [[ ${status} -ne 0 ]]; then
    echo ""
    fail "Pure-Rust verification failed (see above). Policy: scripts/pure_rust_allowlist.txt"
    exit 1
fi

echo "${GREEN}[SUCCESS]${RESET} No forbidden vendored C libraries in the dependency graph."
echo "          ${CHECKED_COUNT} native-code crates checked; all are allowlisted."
