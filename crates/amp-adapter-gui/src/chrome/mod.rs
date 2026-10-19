//! Self-drawn classic window chrome: frameless viewports, a title strip with minimize, shade and
//! close buttons, and app-driven dragging with live magnetic snapping (ADR-002).

#![forbid(unsafe_code)]

pub mod drag;
pub mod handles;
pub mod layout;
pub mod resize;
pub mod zoom;

use crate::app_state::MultiplayGuiApp;
use crate::docking::{DockRect, WindowId};
use drag::DragSession;
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2, ViewportBuilder,
    ViewportCommand, ViewportId,
};
use layout::{Bounds, ChromeHit, ChromeLayout, TITLE_HEIGHT};

/// Window action requested through the title strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeAction {
    None,
    Close,
    Minimize,
    ToggleShade,
}

/// Native viewport id of each window (the main window is the root viewport).
pub fn viewport_id(window: WindowId) -> ViewportId {
    match window {
        WindowId::Main => ViewportId::ROOT,
        WindowId::Equalizer => ViewportId::from_hash_of("win_eq"),
        WindowId::Playlist => ViewportId::from_hash_of("win_playlist"),
        WindowId::Settings => ViewportId::from_hash_of("win_settings"),
        WindowId::DjConsole => ViewportId::from_hash_of("win_dj"),
    }
}

/// Frameless, non-maximizable viewport: the app draws every pixel of the window.
pub fn classic_viewport(title: &str, resizable: bool) -> ViewportBuilder {
    ViewportBuilder::default()
        .with_title(title)
        .with_decorations(false)
        .with_resizable(resizable)
        .with_maximize_button(false)
        .with_transparent(false)
}

const STRIP_ACTIVE: Color32 = Color32::from_rgb(38, 46, 72);
const STRIP_INACTIVE: Color32 = Color32::from_rgb(30, 33, 42);
const GRIP_ACTIVE: Color32 = Color32::from_rgb(214, 170, 72);
const GRIP_INACTIVE: Color32 = Color32::from_rgb(86, 88, 96);
const TEXT_ACTIVE: Color32 = Color32::from_rgb(238, 240, 246);
const TEXT_INACTIVE: Color32 = Color32::from_rgb(140, 144, 156);

/// Draws the title strip at the top of the current viewport and runs dragging for it.
///
/// `full` windows get minimize and shade buttons; overlays only close.
pub fn title_bar(
    app: &mut MultiplayGuiApp,
    ctx: &egui::Context,
    window: WindowId,
    title: &str,
    full: bool,
    snaps: bool,
) -> ChromeAction {
    let scale = 1.0;
    let mut action = ChromeAction::None;
    egui::TopBottomPanel::top(egui::Id::new(("chrome", title)))
        .exact_height(TITLE_HEIGHT * scale)
        .frame(egui::Frame::none())
        .show_separator_line(false)
        .show(ctx, |ui| {
            let strip = ui.max_rect();
            let layout = ChromeLayout::new(strip.width(), scale, full);
            let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
            paint_strip(ui.painter(), strip, &layout, title, focused);

            let resp = ui.interact(
                strip,
                egui::Id::new(("chrome_hit", title)),
                Sense::click_and_drag(),
            );
            let local = resp
                .interact_pointer_pos()
                .map(|p| (p.x - strip.min.x, p.y - strip.min.y));
            let hit = local
                .map(|(x, y)| layout.hit(x, y))
                .unwrap_or(ChromeHit::Content);
            if resp.clicked() {
                action = match hit {
                    ChromeHit::Close => ChromeAction::Close,
                    ChromeHit::Minimize => ChromeAction::Minimize,
                    ChromeHit::Shade => ChromeAction::ToggleShade,
                    _ => ChromeAction::None,
                };
            }
            if resp.double_clicked() && hit == ChromeHit::Title && full {
                action = ChromeAction::ToggleShade;
            }
            if resp.drag_started() && hit == ChromeHit::Title && app.window_resize.is_none() {
                if let Some(p) = pointer_screen(ctx) {
                    let windows = app.dock_manager.dockable_windows();
                    app.window_drag = DragSession::start(window, p, &windows);
                }
            }
        });
    drive_drag(app, ctx, window, snaps);
    action
}

/// Pointer position in screen points (window origin + local position).
pub(crate) fn pointer_screen(ctx: &egui::Context) -> Option<(f32, f32)> {
    ctx.input(|i| {
        let origin = i.viewport().inner_rect?.min;
        let p = i.pointer.latest_pos()?;
        Some((origin.x + p.x, origin.y + p.y))
    })
}

/// Monitor work areas available for screen-edge snapping.
pub(crate) fn screens(ctx: &egui::Context) -> Vec<DockRect> {
    // egui reports only the current monitor's size; treat it as the origin monitor when the
    // window lies on it (multi-monitor origins arrive with TASK-200).
    let (size, outer) = ctx.input(|i| (i.viewport().monitor_size, i.viewport().outer_rect));
    match (size, outer) {
        (Some(s), Some(o)) if o.min.x >= -1.0 && o.min.x < s.x && o.min.y < s.y => {
            vec![DockRect::new(0.0, 0.0, s.x, s.y)]
        }
        _ => Vec::new(),
    }
}

fn drive_drag(app: &mut MultiplayGuiApp, ctx: &egui::Context, window: WindowId, snaps: bool) {
    let Some(session) = app.window_drag.as_ref().filter(|s| s.window == window) else {
        return;
    };
    if !ctx.input(|i| i.pointer.primary_down()) {
        app.window_drag = None;
        app.dock_manager.rebuild_docks_from_contact();
        return;
    }
    let Some(p) = pointer_screen(ctx) else {
        return;
    };
    let windows = app.dock_manager.dockable_windows();
    let (screens, distance) = if snaps {
        (screens(ctx), app.dock_manager.snap_threshold)
    } else {
        (Vec::new(), 0.0)
    };
    let others = if snaps { windows } else { Vec::new() };
    for (id, rect) in session.positions(p, &others, &screens, distance) {
        let old = app.dock_manager.get_rect(id);
        if (old.x - rect.x).abs() > 0.25 || (old.y - rect.y).abs() > 0.25 {
            app.dock_manager.set_rect(id, rect);
            ctx.send_viewport_cmd_to(
                viewport_id(id),
                ViewportCommand::OuterPosition(Pos2::new(rect.x, rect.y)),
            );
        }
    }
    ctx.request_repaint();
}

/// Records where the OS reports a window, except while the app itself is dragging it.
pub fn track_geometry(app: &mut MultiplayGuiApp, ctx: &egui::Context, window: WindowId) {
    // While the app moves or resizes windows it is the source of truth; OS reports lag behind.
    if app.window_drag.is_some() || app.window_resize.is_some() || app.geometry_settle > 0 {
        return;
    }
    let geometry = ctx.input(|i| {
        let outer = i.viewport().outer_rect?;
        let inner = i.viewport().inner_rect.unwrap_or(outer);
        Some(DockRect::new(
            outer.min.x,
            outer.min.y,
            inner.width(),
            inner.height(),
        ))
    });
    if let Some(rect) = geometry {
        app.dock_manager.set_rect(window, rect);
    }
}

/// Applies a title-strip action to `window`. Closing the main window quits the app.
pub fn apply_action(
    app: &mut MultiplayGuiApp,
    ctx: &egui::Context,
    window: WindowId,
    a: ChromeAction,
) {
    let id = viewport_id(window);
    match a {
        ChromeAction::None => {}
        ChromeAction::Close if window == WindowId::Main => {
            ctx.send_viewport_cmd(ViewportCommand::Close)
        }
        ChromeAction::Close => app.dock_manager.set_visible(window, false),
        ChromeAction::Minimize => ctx.send_viewport_cmd_to(id, ViewportCommand::Minimized(true)),
        ChromeAction::ToggleShade => {
            let rect = app.dock_manager.get_rect(window);
            let height = if app.dock_manager.is_shaded(window) {
                app.shade_restore.remove(&window).unwrap_or(rect.height)
            } else {
                app.shade_restore.insert(window, rect.height);
                TITLE_HEIGHT
            };
            app.dock_manager.toggle_shade(window);
            // Windows docked below follow the bottom edge up and back down.
            handles::resize_window(app, ctx, window, rect.width, height);
        }
    }
}

fn paint_strip(
    painter: &egui::Painter,
    strip: Rect,
    layout: &ChromeLayout,
    title: &str,
    focused: bool,
) {
    let (bg, grip, text) = if focused {
        (STRIP_ACTIVE, GRIP_ACTIVE, TEXT_ACTIVE)
    } else {
        (STRIP_INACTIVE, GRIP_INACTIVE, TEXT_INACTIVE)
    };
    painter.rect_filled(strip, 0.0, bg);
    let font = FontId::monospace(8.0);
    let label = painter.layout_no_wrap(title.to_uppercase(), font.clone(), text);
    let label_w = label.size().x + 12.0;
    let center = strip.center().x;
    let buttons_left = layout.minimize.unwrap_or(layout.close).x + strip.min.x - 4.0;
    // Classic grip ridges either side of the centered title.
    for y in [strip.min.y + 4.0, strip.min.y + 7.0, strip.min.y + 10.0] {
        for (x0, x1) in [
            (strip.min.x + 4.0, center - label_w / 2.0),
            (center + label_w / 2.0, buttons_left),
        ] {
            if x1 > x0 {
                painter.hline(x0..=x1, y, Stroke::new(1.0_f32, grip));
            }
        }
    }
    painter.text(
        strip.center(),
        Align2::CENTER_CENTER,
        title.to_uppercase(),
        font,
        text,
    );
    for (b, glyph) in [
        (layout.minimize, "_"),
        (layout.shade, "^"),
        (Some(layout.close), "x"),
    ] {
        if let Some(b) = b {
            paint_button(painter, strip.min, b, glyph, grip, text);
        }
    }
}

fn paint_button(
    painter: &egui::Painter,
    origin: Pos2,
    b: Bounds,
    glyph: &str,
    edge: Color32,
    text: Color32,
) {
    let r = Rect::from_min_size(
        Pos2::new(origin.x + b.x, origin.y + b.y),
        Vec2::new(b.w, b.h),
    );
    painter.rect_filled(r, 1.0, Color32::from_rgb(20, 22, 30));
    painter.rect_stroke(r, 1.0, Stroke::new(1.0_f32, edge));
    painter.text(
        r.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::monospace(7.0),
        text,
    );
}
