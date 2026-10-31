#!/usr/bin/env bash
# Bundle all Multiplay build artifacts into a clean, dedicated output directory (build/)
# Keeps the repository root clean of .app bundles and launcher scripts.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$WORKSPACE_ROOT/build"

echo "Bundling build artifacts into $BUILD_DIR..."
mkdir -p "$BUILD_DIR/bin"
mkdir -p "$BUILD_DIR/macos/Multiplay.app/Contents/MacOS"
mkdir -p "$BUILD_DIR/macos/Multiplay.app/Contents/Resources"

# 1. Copy CLI/TUI binary
if [ -f "$WORKSPACE_ROOT/target/release/amp-app" ]; then
    cp "$WORKSPACE_ROOT/target/release/amp-app" "$BUILD_DIR/bin/multiplay"
    chmod +x "$BUILD_DIR/bin/multiplay"
elif [ -f "$WORKSPACE_ROOT/target/debug/amp-app" ]; then
    cp "$WORKSPACE_ROOT/target/debug/amp-app" "$BUILD_DIR/bin/multiplay"
    chmod +x "$BUILD_DIR/bin/multiplay"
fi

# 2. Copy GUI binary
if [ -f "$WORKSPACE_ROOT/target/release/multiplay-gui" ]; then
    cp "$WORKSPACE_ROOT/target/release/multiplay-gui" "$BUILD_DIR/bin/multiplay-gui"
    chmod +x "$BUILD_DIR/bin/multiplay-gui"
elif [ -f "$WORKSPACE_ROOT/target/debug/multiplay-gui" ]; then
    cp "$WORKSPACE_ROOT/target/debug/multiplay-gui" "$BUILD_DIR/bin/multiplay-gui"
    chmod +x "$BUILD_DIR/bin/multiplay-gui"
fi

# 3. macOS App executable (prefer native GUI window)
if [ -f "$BUILD_DIR/bin/multiplay-gui" ]; then
    cp "$BUILD_DIR/bin/multiplay-gui" "$BUILD_DIR/macos/Multiplay.app/Contents/MacOS/Multiplay"
    chmod +x "$BUILD_DIR/macos/Multiplay.app/Contents/MacOS/Multiplay"
elif [ -f "$BUILD_DIR/bin/multiplay" ]; then
    cp "$BUILD_DIR/bin/multiplay" "$BUILD_DIR/macos/Multiplay.app/Contents/MacOS/Multiplay"
    chmod +x "$BUILD_DIR/macos/Multiplay.app/Contents/MacOS/Multiplay"
fi

# 4. Bundle macOS Info.plist
if [ -f "$WORKSPACE_ROOT/scripts/Info.plist" ]; then
    cp "$WORKSPACE_ROOT/scripts/Info.plist" "$BUILD_DIR/macos/Multiplay.app/Contents/Info.plist"
fi
cp "$WORKSPACE_ROOT/assets/icon/multiplay.icns" "$BUILD_DIR/macos/Multiplay.app/Contents/Resources/Multiplay.icns"
# The license bundle Preferences -> About opens (built in CI; locally only if cargo-about is installed).
if command -v cargo-about >/dev/null 2>&1; then
    (cd "$WORKSPACE_ROOT" && cargo about generate --locked about.hbs \
        -o "$BUILD_DIR/macos/Multiplay.app/Contents/Resources/THIRD_PARTY_LICENSES.html")
fi

# 5. Create Multiplay.command inside build/macos/
cat << 'EOF' > "$BUILD_DIR/macos/Multiplay.command"
#!/usr/bin/env bash
# Double-clickable launcher located inside build/macos/
DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$DIR/../.." && pwd)"
if [ -x "$WORKSPACE_ROOT/build/bin/multiplay-gui" ]; then
    exec "$WORKSPACE_ROOT/build/bin/multiplay-gui" "$@"
elif [ -x "$WORKSPACE_ROOT/build/bin/multiplay" ]; then
    exec "$WORKSPACE_ROOT/build/bin/multiplay" "$@"
elif [ -x "$WORKSPACE_ROOT/target/release/multiplay-gui" ]; then
    exec "$WORKSPACE_ROOT/target/release/multiplay-gui" "$@"
else
    echo "Multiplay binary not found in build directory. Run 'make bundle' first."
    exit 1
fi
EOF
chmod +x "$BUILD_DIR/macos/Multiplay.command"

echo "[SUCCESS] Build artifacts bundled successfully:"
echo "  • CLI / TUI Binary:  $BUILD_DIR/bin/multiplay"
echo "  • Native GUI Binary: $BUILD_DIR/bin/multiplay-gui"
echo "  • macOS App Bundle:  $BUILD_DIR/macos/Multiplay.app"
echo "  • macOS Command:     $BUILD_DIR/macos/Multiplay.command"
