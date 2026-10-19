//! Resize handles (right edge, bottom edge, bottom-right grip), app-driven resizing, and the
//! shared code that applies size changes while keeping every window apart.

#![forbid(unsafe_code)]

use super::resize::ResizeSession;
use super::{pointer_screen, screens, viewport_id};
use crate::app_state::MultiplayGuiApp;
use crate::docking::{clamp_growth, resize_with_attached, DockRect, WindowId};
use eframe::egui::{
    self, Color32, CursorIcon, LayerId, Order, Pos2, Stroke, Vec2, ViewportCommand,
};

/// Width of the grabbable right and bottom edges.
pub const EDGE: f32 = 5.0;
/// Side of the square bottom-right grip.
pub const GRIP: f32 = 12.0;

/// Which edges a pointer at `pos` (window-local) would resize: `(right, bottom)`.
pub fn hit_zone(pos: Pos2, size: Vec2) -> Option<(bool, bool)> {
    let in_grip = pos.x >= size.x - GRIP && pos.y >= size.y - GRIP;
    let right = pos.x >= size.x - EDGE;
    let bottom = pos.y >= size.y - EDGE;
    if pos.x < 0.0 || pos.y < 0.0 || pos.x > size.x || pos.y > size.y {
        None
    } else if in_grip {
        Some((true, true))
    } else if right || bottom {
        Some((right, bottom))
    } else {
        None
    }
}

/// Draws the grip, shows resize cursors, and starts/drives resizing for `window`.
pub fn resize_handles(app: &mut MultiplayGuiApp, ctx: &egui::Context, window: WindowId) {
    if app.dock_manager.is_shaded(window) {
        return;
    }
    let size = ctx.content_rect().size();
    paint_grip(ctx, window, size);
    let busy = app.window_drag.is_some() || app.window_resize.is_some();
    if !busy {
        let zone = ctx
            .input(|i| i.pointer.latest_pos())
            .and_then(|p| hit_zone(p, size));
        if let Some((right, bottom)) = zone {
            ctx.set_cursor_icon(match (right, bottom) {
                (true, true) => CursorIcon::ResizeNwSe,
                (true, false) => CursorIcon::ResizeHorizontal,
                _ => CursorIcon::ResizeVertical,
            });
            if ctx.input(|i| i.pointer.primary_pressed()) {
                if let Some(p) = pointer_screen(ctx) {
                    let start = app.dock_manager.get_rect(window);
                    let session = ResizeSession::start(window, right, bottom, p, start)
                        .with_min(app.min_size(window));
                    app.window_resize = Some(session);
                }
            }
        }
    }
    drive_resize(app, ctx, window);
}

fn drive_resize(app: &mut MultiplayGuiApp, ctx: &egui::Context, window: WindowId) {
    let Some(session) = app.window_resize.clone().filter(|s| s.window == window) else {
        return;
    };
    if !ctx.input(|i| i.pointer.primary_down()) {
        app.window_resize = None;
        app.dock_manager.rebuild_docks_from_contact();
        return;
    }
    let Some(p) = pointer_screen(ctx) else {
        return;
    };
    let windows = layout_including(app, window);
    let others: Vec<DockRect> = windows
        .iter()
        .filter(|(id, _)| *id != window)
        .map(|(_, r)| *r)
        .collect();
    let snap = app.dock_manager.snap_threshold;
    let (w, h) = session.target_size(p, &others, &screens(ctx), snap);
    let (w, h) = clamp_growth(window, w, h, &windows);
    let moves = resize_with_attached(window, w, h, &windows);
    apply_moves(app, ctx, window, &moves);
    ctx.request_repaint();
}

/// Dockable windows plus `window` itself (the settings overlay is not dockable but resizable).
fn layout_including(app: &MultiplayGuiApp, window: WindowId) -> Vec<(WindowId, DockRect)> {
    let mut windows = app.dock_manager.dockable_windows();
    if !windows.iter().any(|(id, _)| *id == window) {
        windows = vec![(window, app.dock_manager.get_rect(window))];
    }
    windows
}

/// Sets `window` to `w` x `h` and moves other windows as needed so nothing overlaps.
pub fn resize_window(
    app: &mut MultiplayGuiApp,
    ctx: &egui::Context,
    window: WindowId,
    w: f32,
    h: f32,
) {
    let windows = layout_including(app, window);
    let moves = resize_with_attached(window, w, h, &windows);
    apply_moves(app, ctx, window, &moves);
}

/// Grows `window` when its content spilled past the window (runtime never-clip guard).
pub fn grow_to_fit(app: &mut MultiplayGuiApp, ctx: &egui::Context, window: WindowId, spill: Vec2) {
    if spill.x <= 0.5 && spill.y <= 0.5 || app.window_resize.is_some() || app.geometry_settle > 0 {
        return;
    }
    let r = app.dock_manager.get_rect(window);
    resize_window(
        app,
        ctx,
        window,
        (r.width + spill.x).ceil(),
        (r.height + spill.y).ceil(),
    );
}

/// Records new rects and sends them to the native windows: a size for `resized`, positions for
/// every other moved window.
pub fn apply_moves(
    app: &mut MultiplayGuiApp,
    ctx: &egui::Context,
    resized: WindowId,
    moves: &[(WindowId, DockRect)],
) {
    for (id, rect) in moves {
        let old = app.dock_manager.get_rect(*id);
        app.dock_manager.set_rect(*id, *rect);
        let vp = viewport_id(*id);
        if *id == resized
            && ((old.width - rect.width).abs() > 0.25 || (old.height - rect.height).abs() > 0.25)
        {
            ctx.send_viewport_cmd_to(
                vp,
                ViewportCommand::InnerSize(Vec2::new(rect.width, rect.height)),
            );
        }
        if (old.x - rect.x).abs() > 0.25 || (old.y - rect.y).abs() > 0.25 {
            ctx.send_viewport_cmd_to(
                vp,
                ViewportCommand::OuterPosition(Pos2::new(rect.x, rect.y)),
            );
        }
    }
}

fn paint_grip(ctx: &egui::Context, window: WindowId, size: Vec2) {
    let painter = ctx.layer_painter(LayerId::new(
        Order::Foreground,
        egui::Id::unique(("grip", window)),
    ));
    let stroke = Stroke::new(1.0_f32, Color32::from_rgb(110, 118, 132));
    for i in 1..=3 {
        let d = i as f32 * 3.0;
        painter.line_segment(
            [
                Pos2::new(size.x - d - 1.0, size.y - 1.0),
                Pos2::new(size.x - 1.0, size.y - d - 1.0),
            ],
            stroke,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vec2 = Vec2::new(480.0, 300.0);

    #[test]
    fn grip_edges_and_content_are_told_apart() {
        assert_eq!(hit_zone(Pos2::new(475.0, 295.0), SIZE), Some((true, true)));
        assert_eq!(hit_zone(Pos2::new(478.0, 100.0), SIZE), Some((true, false)));
        assert_eq!(hit_zone(Pos2::new(200.0, 298.0), SIZE), Some((false, true)));
        assert_eq!(hit_zone(Pos2::new(200.0, 100.0), SIZE), None);
        assert_eq!(hit_zone(Pos2::new(500.0, 100.0), SIZE), None);
    }
}
