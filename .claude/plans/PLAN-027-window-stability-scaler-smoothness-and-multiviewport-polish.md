# PLAN-027: Window Stability, Scaler Smoothness & Multi-Viewport Polish (Flicker-Free Industry-Grade Architecture)

## Overview & User Requirement
The user reported:
> *"the single windows and the content is very flickering when scaling the windows, also they jump around and the scale to the screen full hight randomly, some times they just get black. this is very buggy and laggy! deeply check why this happens and make sure this get fixed! we need 100% industry grate software. maybe check e.g. artcraft software, like vectorcraft we have it locally in /Users/rex-fab-alt/Documents/private/vectorcraft"*

## Root Cause Analysis
1. **Positive Feedback Loop between Size and Builder:**
   In `render_equalizer_viewport`, `render_playlist_viewport`, etc., the code was reading `outer_rect` from `i.viewport().outer_rect`, which on macOS includes window borders and the native title bar (~28px). This outer height was saved into `dock_manager`, and then passed to `builder.with_inner_size(pl_rect.width, pl_rect.height)` on every single frame. This created a compounding positive feedback loop:
   `outer_rect` (+28px title bar) $\to$ set as `inner_size` $\to$ new `outer_rect` (+another 28px) $\to$ set as `inner_size` $\to$ window explodes to full screen height in seconds!
2. **Per-Frame ViewportBuilder Commands Fighting Native Window Manager:**
   `with_inner_size` and `with_position` were specified in `ViewportBuilder` on every frame. When the user attempted to scale/resize a window by dragging its border, the OS resized the window, but egui's builder immediately sent a competing `ViewportCommand::InnerSize` to force it back. This conflict triggered rapid, 60 FPS swapchain recreations in WGPU/Metal, resulting in severe flickering, dropped frames, black buffers, and cursor fighting.
3. **Continuous OuterPosition Dispatch During Mouse Drag:**
   `compute_snap` dispatched `ViewportCommand::OuterPosition` while the user's cursor was actively holding the title bar. On macOS, the WindowServer owns the mouse grab during window drag; programmatic window moves during an active drag caused the window to violently oscillate between the cursor and the snap point.
4. **Unconditional 60 FPS Repaint Overload:**
   Repainting all 5 native viewports at 60 FPS even when playback was stopped/idle caused GPU and swapchain pressure across Metal surfaces.

## Technical Architecture & Fixes
1. **One-Time Spawn Parameters (`with_inner_size` & `with_position`):**
   `with_inner_size` and `with_position` are ONLY set once upon window initialization (`!initialized_viewports.contains(&id)`). Once a window is alive, its `ViewportBuilder` specifies `None` for size and position, allowing the operating system to handle dragging and scaling natively with zero swapchain thrashing.
2. **Inner/Outer Coordinate Disambiguation:**
   Position is tracked from `outer_rect.min`, while dimension is tracked from `inner_rect.size()`. This completely breaks the positive feedback loop.
3. **Drag-Release Magnetic Snapping:**
   During active pointer press (`i.pointer.any_down()`), no programmatic repositioning is executed. Magnetic snap alignment is only computed once when the user releases the mouse button (`was_dragging` transition).
4. **Event-Driven / Playback-Driven Repaint:**
   60 FPS continuous repaint is only scheduled when audio is actively playing (`audio_driver.is_playing`). When stopped/paused, UI updates are purely event-driven, eliminating idle CPU/GPU usage.
5. **Native VSync & Hardware Options:**
   Configured `vsync: true` and `multisampling: 0` for smooth Metal presentation.

---

## Tasks Breakdown
- **TASK-083**: Eliminate ViewportBuilder per-frame size/position commands and positive feedback loop
- **TASK-084**: Drag-Release Magnetic Snap Stabilization and WindowServer Decoupling
- **TASK-085**: Conditional Event-Driven/Playback-Driven Repaint Scheduling & VSync Hardware Tuning
- **TASK-086**: Comprehensive Testing, Verification, Zero Clippy Warnings, and Build Packaging
