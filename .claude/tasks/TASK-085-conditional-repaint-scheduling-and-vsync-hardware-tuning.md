# TASK-085: Conditional Repaint Scheduling & VSync Hardware Tuning

## Plan Reference
- Plan: [PLAN-027](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-027-window-stability-scaler-smoothness-and-multiviewport-polish.md)
- Role: `gui_engineering`

## Objectives
1. Replace unconditional `ctx.request_repaint_after(16ms)` with conditional scheduling gated on `self.audio_driver.is_playing.load(Ordering::Relaxed)`.
2. Ensure idle state is purely event-driven, eliminating GPU render pass congestion across the multi-viewport surface set.
3. Configure `NativeOptions` with `vsync: true` and `multisampling: 0` for consistent CAMetalLayer frame pacing.

## Verification Command
`cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui`

## Status
- **Status:** COMPLETED
- **Outcome:** Repaint scheduling is conditioned on `audio_driver.is_playing`, rendering idle states strictly event-driven. Configured `vsync: true` and `multisampling: 0` in `main.rs` for flicker-free Metal rendering. Verified with all tests passing.

