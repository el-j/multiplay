#!/usr/bin/env bash
# Launches Multiplay Pure-Rust Native Desktop GUI (Metal GPU-accelerated)
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
exec "$DIR/target/release/multiplay-gui" "$@"
