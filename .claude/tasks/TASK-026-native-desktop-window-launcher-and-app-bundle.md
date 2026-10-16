---
id: TASK-026
title: Standalone Native Desktop Window Launcher, macOS Application Bundle & Scripts
planId: PLAN-009
status: completed
role: frontend_gui
verificationCommand: ./scripts/launch_desktop_window.sh && test -x Multiplay.command && test -f Multiplay.app/Contents/Info.plist
---

# TASK-026: Standalone Native Desktop Window Launcher, macOS Application Bundle & Scripts

## Description
Provide native window launching mechanisms so Multiplay opens its own dedicated floating desktop window:
1. `scripts/launch_desktop_window.sh`: Executable shell script using AppleScript / `osascript` to launch a new, floating, styled native macOS window sized to retro Winamp dimensions ($80 \times 30$), titled `"Multiplay Pure-Rust Audiophile Player"`.
2. `Multiplay.command`: Root-level executable for double-click launching directly from macOS Finder.
3. `Multiplay.app`: macOS Application bundle with `Info.plist` and executable wrapper for system integration.

## Verification
```bash
./scripts/launch_desktop_window.sh
test -x Multiplay.command
test -f Multiplay.app/Contents/Info.plist
```
