---
id: TASK-026
title: Standalone Native Desktop Window Launcher, macOS Application Bundle & Scripts
planId: PLAN-009
status: completed
role: frontend_gui
verificationCommand: ./scripts/bundle_build.sh && test -x build/macos/Multiplay.command && test -f build/macos/Multiplay.app/Contents/Info.plist && test -x build/bin/multiplay && test -x build/bin/multiplay-gui
---

# TASK-026: Standalone Native Desktop Window Launcher, macOS Application Bundle & Scripts

## Description
Provide native window launching mechanisms bundled neatly into a dedicated `build/` output directory:
1. `build/bin/`: Standalone binaries for CLI (`multiplay`) and Native GPU GUI (`multiplay-gui`).
2. `build/macos/Multiplay.command`: Executable launcher inside `build/macos/` for double-click launching from Finder without polluting repo root.
3. `build/macos/Multiplay.app`: macOS Application bundle with `Info.plist` and native GUI binary inside `Contents/MacOS/Multiplay`.
4. `scripts/launch_desktop_window.sh`: Executable helper script to launch desktop windows.

## Verification
```bash
./scripts/bundle_build.sh
test -x build/macos/Multiplay.command
test -f build/macos/Multiplay.app/Contents/Info.plist
test -x build/bin/multiplay
test -x build/bin/multiplay-gui
test ! -e Multiplay.app
test ! -e Multiplay.command
```

