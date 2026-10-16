#!/usr/bin/env bash
# Standalone Native Desktop Window Launcher for Multiplay (macOS)
# Spawns Multiplay in its own dedicated, floating native OS window with retro Winamp geometry.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building Multiplay binary..."
cd "$WORKSPACE_ROOT"
cargo build --release -p amp-app

BINARY="$WORKSPACE_ROOT/target/release/amp-app"

if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    exit 1
fi

echo "Launching Multiplay in a dedicated native desktop window..."

osascript <<EOF
tell application "Terminal"
    activate
    set newWindow to do script "cd '$WORKSPACE_ROOT' && '$BINARY'"
    tell newWindow
        set number of columns to 80
        set number of rows to 30
        set custom title to "Multiplay Pure-Rust Audiophile Player"
    end tell
end tell
EOF

echo "Multiplay desktop window launched successfully."
