# AUDIT-09 — Build, CI, Release & Supply Chain

Part of [AUDIT-2026-10-17](README.md).

## Local results (macOS arm64, 2026-10-17)

| Check | Result |
| :--- | :--- |
| `./scripts/verify_pure_rust.sh` | ✅ "SUCCESS" (but see BLD-01) |
| `cargo build --workspace` | ✅ (future-incompat warning: `block v0.1.6`) |
| `cargo test --workspace` | ✅ ~145 tests, 0 failed (needs audio hardware, see QA-01) |
| `cargo clippy --workspace --all-targets` | ✅ 0 warnings |
| `cargo fmt --all -- --check` | ❌ **47 diffs** |
| GitHub Actions (last 2 runs on `main`) | ❌ **failure**: `verify_pure_rust.sh` found `libloading` on Linux |

## BLD-01 — Pure-Rust verifier is unsound · P1
**Evidence:** `cargo tree --edges no-build -i <crate>` (`scripts/verify_pure_rust.sh:18`) **excludes build-dependencies**, which is exactly where `cc`, `pkg-config` and `bindgen` live, and it only inspects the **host** target. With `--target all -e normal,build`:
- `pkg-config` ← `alsa-sys` ← `alsa` ← `cpal` (links **libasound** on Linux)
- `cc` ← `ring` (rustls), `android-activity`/`winit`, `oboe-sys`
- `bindgen` ← `coreaudio-sys` (macOS)
- `libloading` ← wgpu/winit on Linux (this is what fails CI)

The local "SUCCESS" is a false negative. CI's Linux runner gives the true positive.
**Fix:** Redefine the policy (ORC-05). Rewrite the verifier to scan all targets including build deps, against an **explicit allowlist** of OS-binding `-sys` crates, failing on anything that vendors C (openssl-src, libsqlite3-sys, ffmpeg-sys, …). Consider `rustls` with the `ring` provider as allow-listed, or evaluate pure-Rust crypto providers. → TASK-113

## BLD-02 — CI quality gate can't pass · P0
**Evidence:** `cargo fmt --check` fails locally with 47 diffs. The verifier fails on Linux. The Ubuntu jobs would also need `libasound2-dev` (and `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev` and others for eframe/winit), which the workflow never installs.
**Fix:** `cargo fmt`, install the system deps, and run the gate on all 3 OSes. → TASK-120

## BLD-03 — CI ignores policy-critical checks · P2
**Evidence:** No `cargo deny` (licenses, advisories, duplicate versions), no `cargo audit`, no MSRV, no `--locked`, and no doc warnings as errors (`RUSTDOCFLAGS=-D warnings`). The mutation-testing step only checks that `mutants.toml` exists.
**Fix:** → TASK-124

## BLD-04 — Release workflow is broken · P0 for shipping
**Evidence:** The macOS packaging does `cp Multiplay.command staging/` (`.github/workflows/release.yml:56`), but the file isn't in the repo (it was gitignored or deleted), so the macOS job fails. The `.app` built in CI has no `Info.plist`, icon, signing or notarization. Linux and Windows ship raw binaries in tar/zip with no installer, desktop entry or icon. `|| true` hides a missing GUI binary.
**Fix:** → TASK-123 (fix now), TASK-181 (proper packaging)

## BLD-05 — Dependency hygiene · P2
**Evidence:**
- `block 0.1.6` future-incompat warning (from `objc` via the eframe stack).
- `zip = "9.0"` with `default-features = false` (fine) but no size limits (SEC-07).
- `bincode 1.3` (DAT-03).
- `rand 0.8`, `thiserror 1.0`, and `lofty 0.21` duplicated across crates without workspace pinning.
- Two copies of the async runtime setup.
- `target/` is **27 GB** (no `cargo clean` policy; consider `sccache` and `CARGO_INCREMENTAL=0` in CI).
**Fix:** Workspace-level dependency pinning, Dependabot/Renovate, and the cargo-deny `bans` section. → TASK-124

## BLD-06 — No signed, installable artifacts · P1
**Fix:** macOS `.app` + DMG with Developer ID signing and notarization (`apple-codesign`, a pure-Rust `rcodesign`, fits the ethos). Windows MSI (`cargo-wix`) + Authenticode. Linux AppImage + `.deb` + Flatpak manifest. Reproducible version stamping. → TASK-181

## BLD-07 — Two binaries, unclear entry point · P2
**Evidence:** `amp-app` (TUI) and `multiplay-gui` (inside the adapter crate). Users and packagers expect one `multiplay` command.
**Fix:** A single `multiplay` binary in `amp-app`: GUI by default, `--tui`, and `--headless --remote` for a home-server mode. → TASK-180
