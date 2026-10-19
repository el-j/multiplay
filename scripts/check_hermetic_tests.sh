#!/usr/bin/env bash
# Runs the workspace tests with HOME pointed at an empty sandbox and fails if any test
# wrote multiplay state there (TASK-122). Cargo/rustup keep using the real toolchain dirs.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REAL_HOME="${HOME}"
SANDBOX="$(mktemp -d)"
# Best-effort: on Windows the OS keeps writing its own caches under the profile dir.
trap 'rm -rf "${SANDBOX}" 2>/dev/null || true' EXIT

export CARGO_HOME="${CARGO_HOME:-${REAL_HOME}/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-${REAL_HOME}/.rustup}"
export HOME="${SANDBOX}"
export USERPROFILE="${SANDBOX}"

cd "${ROOT_DIR}"
cargo test --workspace --locked "$@"

leaked="$(find "${SANDBOX}" -mindepth 1 \( -path "${SANDBOX}/.multiplay*" -o -path "${SANDBOX}/.config/multiplay*" -o -path "${SANDBOX}/Music*" \) -print 2>/dev/null || true)"
if [[ -n "${leaked}" ]]; then
    echo "[FAIL] Tests wrote user state into HOME:" >&2
    echo "${leaked}" >&2
    exit 1
fi
echo "[PASS] Tests left no multiplay state in HOME."
