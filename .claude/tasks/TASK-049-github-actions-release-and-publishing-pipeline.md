---
id: TASK-049
title: GitHub Actions Automated Cross-Platform Release & Publishing Pipeline
planId: PLAN-015
status: completed
role: devops_and_release
verificationCommand: test -f .github/workflows/release.yml
---

# TASK-049: GitHub Actions Automated Cross-Platform Release & Publishing Pipeline

## Description
Create `.github/workflows/release.yml` automating binary build, packaging, checksum generation, and GitHub release creation upon tag push (`v*`):
1. Multi-Target Compilation Matrix:
   - macOS aarch64 (Apple Silicon) & x86_64: Packages `Multiplay.app.tar.gz` and `multiplay` binary.
   - Linux x86_64: Packages `multiplay-linux-x86_64.tar.gz` containing CLI and GUI binaries.
   - Windows x86_64: Packages `multiplay-windows-x86_64.zip` containing `multiplay.exe` and `multiplay-gui.exe`.
2. SHA256 Checksums: Generates `SHA256SUMS.txt` for all release assets.
3. Automated GitHub Release: Publishes assets, creates release changelog, and tags release.

## Verification
```bash
test -f .github/workflows/release.yml
```
