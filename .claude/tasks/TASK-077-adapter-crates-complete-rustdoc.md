---
id: TASK-077
title: Adapter Crates & Application Shells Complete Rustdoc Documentation
planId: PLAN-025
status: completed
role: documentation
verificationCommand: cargo doc --workspace --no-deps
---

# TASK-077: Adapter Crates & Application Shells Complete Rustdoc Documentation

## Description
Document 100% of public types, methods, and modules across all driving and driven adapter crates:
1. Audio & Format Adapters: Document `amp-adapter-cpal` (CoreAudio/ALSA/WASAPI integration, real-time audio callback safety guarantees) and `amp-adapter-decoder` (Symphonia format support, demuxing, sample conversion, seeking).
2. Remote & Storage Adapters: Document `amp-adapter-storage` (redb schema, ACID transactions), `amp-adapter-youtube`, `amp-adapter-spotify`, `amp-adapter-soulseek`, and `amp-adapter-p2p`.
3. Presentation Shells: Document `amp-adapter-tui` (Ratatui modular components, key bindings), `amp-adapter-gui` (egui windows, magnetic docking system, theme tokens), and `amp-app` (CLI runner and composition root).
4. Zero Missing Docs: Ensure all crates generate complete HTML documentation without warnings.

## Verification
```bash
cargo doc --workspace --no-deps
```

## Acceptance Criteria
- Full workspace rustdoc generates cleanly with zero broken links or missing type docs.
- Clear module-level architectural summaries in every crate root.
- All adapter contracts match documented port semantics.
