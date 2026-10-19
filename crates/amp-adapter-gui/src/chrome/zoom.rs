//! Double size (Ctrl+D, Cmd+D on macOS): every window and its content at 2x, scaled around the
//! main window like classic Winamp. Built on egui's zoom factor, so text stays crisp.

#![forbid(unsafe_code)]

use super::viewport_id;
use crate::app_state::MultiplayGuiApp;
use crate::docking::{rescale_layout, WindowId};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, Pos2, Vec2, ViewportCommand};

pub const DOUBLE_SIZE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
/// Frames during which OS geometry reports are ignored after a zoom change: the native windows
/// take a few frames to reach their new size, and their reports in between are stale.
pub const SETTLE_FRAMES: u8 = 12;

/// The zoom factor double size toggles to from `current`.
pub fn toggled(current: f32) -> f32 {
    if current > 1.5 {
        1.0
    } else {
        2.0
    }
}

/// Handles the shortcut in whichever window has keyboard focus.
pub fn shortcut(ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_shortcut(&DOUBLE_SIZE)) {
        ctx.set_zoom_factor(toggled(ctx.zoom_factor()));
    }
}

/// Once per frame, before any geometry is read: when the zoom changed (double size, or the
/// standard zoom keys), re-express the layout in the new units and move every window there.
pub fn sync(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    app.geometry_settle = app.geometry_settle.saturating_sub(1);
    if app.geometry_settle > 0 {
        ctx.request_repaint();
    }
    let zoom = ctx.zoom_factor();
    if (zoom - app.applied_zoom).abs() < 1e-3 {
        return;
    }
    let all: Vec<_> = app
        .dock_manager
        .windows
        .iter()
        .map(|(id, r)| (*id, *r))
        .collect();
    for (id, r) in rescale_layout(&all, WindowId::Main, app.applied_zoom, zoom) {
        app.dock_manager.set_rect(id, r);
        if app.dock_manager.is_visible(id) {
            let vp = viewport_id(id);
            ctx.send_viewport_cmd_to(vp, ViewportCommand::InnerSize(Vec2::new(r.width, r.height)));
            ctx.send_viewport_cmd_to(vp, ViewportCommand::OuterPosition(Pos2::new(r.x, r.y)));
        }
    }
    app.applied_zoom = zoom;
    app.geometry_settle = SETTLE_FRAMES;
    app.window_drag = None;
    app.window_resize = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_size_toggles_between_1x_and_2x() {
        assert_eq!(toggled(1.0), 2.0);
        assert_eq!(toggled(2.0), 1.0);
        // From a keyboard zoom step, the shortcut goes to 2x or back to 1x.
        assert_eq!(toggled(1.2), 2.0);
        assert_eq!(toggled(1.8), 1.0);
    }
}
