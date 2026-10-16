# TASK-083: Eliminate ViewportBuilder Per-Frame Size/Position Commands and Positive Feedback Loop

## Plan Reference
- Plan: [PLAN-027](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-027-window-stability-scaler-smoothness-and-multiviewport-polish.md)
- Role: `gui_engineering`

## Objectives
1. Refactor satellite window rendering in `crates/amp-adapter-gui/src/app_render.rs` so that `with_inner_size` and `with_position` are ONLY set once upon initialization (`!initialized_viewports.contains(&id)`).
2. For subsequent frames, ensure `ViewportBuilder` passes `None` for size and position, eliminating all conflicting `ViewportCommand::InnerSize` and `ViewportCommand::OuterPosition` events during scaling.
3. Disambiguate `outer_rect` vs `inner_rect`: update `dock_manager` dimensions using `inner_rect.size()` to permanently stop the compounding title bar growth feedback loop.

## Verification Command
`cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui`

## Status
- **Status:** COMPLETED
- **Outcome:** One-time spawn initialization implemented in `prepare_viewport_builder`. Subsequent frames do not provide conflicting inner sizes or positions. `inner_rect.size()` is used for `dock_manager` dimensions, completely eliminating the macOS title bar positive feedback growth loop. Verified with 11 passed tests and 0 compilation errors.

