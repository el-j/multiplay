//! Modular satellite viewport rendering and docking snap handlers for Multiplay GUI.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_panel::DjConsolePanel;
use crate::docking::{DockRect, WindowId};
use crate::equalizer_widget::EqualizerPanel;
use crate::playlist_window::render_playlist_window_content;
use crate::settings_panel::SettingsGuiPanel;
use eframe::egui::{self, Color32, Pos2, Vec2, ViewportBuilder, ViewportCommand, ViewportId};

/// Renders all visible satellite modular windows in their own independent native OS viewports.
pub fn render_satellite_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    render_equalizer_viewport(app, ctx);
    render_playlist_viewport(app, ctx);
    render_settings_viewport(app, ctx);
    render_dj_viewport(app, ctx);
}

/// Helper to prepare a ViewportBuilder that only specifies position and size ONCE on first spawn.
fn prepare_viewport_builder(
    app: &mut MultiplayGuiApp,
    id: WindowId,
    title: &str,
    min: Vec2,
) -> ViewportBuilder {
    let mut b = ViewportBuilder::default()
        .with_title(title)
        .with_min_inner_size(min)
        .with_resizable(true);
    if !app.initialized_viewports.contains(&id) {
        let r = app.dock_manager.get_rect(id);
        b = b
            .with_inner_size(Vec2::new(r.width, r.height))
            .with_position(Pos2::new(r.x, r.y));
        app.initialized_viewports.insert(id);
    }
    b
}

/// Tracks viewport desktop geometry and performs magnetic docking upon drag release.
fn handle_satellite_geometry_and_snap(
    app: &mut MultiplayGuiApp,
    ctx: &egui::Context,
    id: WindowId,
    default_inner: Vec2,
    enable_snap: bool,
) {
    let Some(outer) = ctx.input(|i| i.viewport().outer_rect) else {
        return;
    };
    let inner_size = ctx
        .input(|i| i.viewport().inner_rect)
        .map(|r| r.size())
        .unwrap_or(default_inner);
    let cur = DockRect::new(outer.min.x, outer.min.y, inner_size.x, inner_size.y);
    app.dock_manager.set_rect(id, cur);

    if enable_snap {
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        if !pointer_down && app.was_dragging.remove(&id) {
            let (snapped, dock_info) = app.dock_manager.compute_snap(id, cur);
            if (snapped.x - cur.x).abs() > 0.5 || (snapped.y - cur.y).abs() > 0.5 {
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(
                    snapped.x, snapped.y,
                )));
                app.dock_manager.set_rect(id, snapped);
            }
            if let Some((parent, edge)) = dock_info {
                app.dock_manager.dock(id, parent, edge);
            } else {
                app.dock_manager.undock(id);
            }
        } else if pointer_down {
            app.was_dragging.insert(id);
        }
    }
}

fn render_equalizer_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if !app.dock_manager.is_visible(WindowId::Equalizer) {
        app.initialized_viewports.remove(&WindowId::Equalizer);
        app.was_dragging.remove(&WindowId::Equalizer);
        return;
    }
    let builder = prepare_viewport_builder(
        app,
        WindowId::Equalizer,
        "WINAMP :: EQUALIZER",
        Vec2::new(380.0, 150.0),
    );
    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("win_eq"),
        builder,
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                app.dock_manager.set_visible(WindowId::Equalizer, false);
            }
            handle_satellite_geometry_and_snap(
                app,
                ctx,
                WindowId::Equalizer,
                Vec2::new(480.0, 155.0),
                true,
            );
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(14, 18, 22))
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| {
                    let prev_preamp = app.eq_preamp;
                    let prev_gains = app.eq_gains;
                    let prev_enabled = app.eq_enabled;

                    EqualizerPanel::new(&mut app.eq_preamp, &mut app.eq_gains, &mut app.eq_enabled)
                        .ui(ui);

                    if (prev_preamp - app.eq_preamp).abs() > 0.005 {
                        app.audio_driver.set_eq_preamp(app.eq_preamp);
                    }
                    if prev_gains != app.eq_gains {
                        for (i, &g) in app.eq_gains.iter().enumerate() {
                            app.audio_driver.set_eq_band(i, g);
                        }
                    }
                    if prev_enabled != app.eq_enabled {
                        app.audio_driver.toggle_eq();
                    }
                });
        },
    );
}

fn render_playlist_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if !app.dock_manager.is_visible(WindowId::Playlist) {
        app.initialized_viewports.remove(&WindowId::Playlist);
        app.was_dragging.remove(&WindowId::Playlist);
        return;
    }
    let builder = prepare_viewport_builder(
        app,
        WindowId::Playlist,
        "WINAMP :: PLAYLIST",
        Vec2::new(380.0, 180.0),
    );
    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("win_playlist"),
        builder,
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                app.dock_manager.set_visible(WindowId::Playlist, false);
            }
            handle_satellite_geometry_and_snap(
                app,
                ctx,
                WindowId::Playlist,
                Vec2::new(480.0, 280.0),
                true,
            );
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(14, 18, 22))
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| {
                    render_playlist_window_content(app, ui);
                });
        },
    );
}

fn render_settings_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if !app.dock_manager.is_visible(WindowId::Settings) {
        app.initialized_viewports.remove(&WindowId::Settings);
        app.was_dragging.remove(&WindowId::Settings);
        return;
    }
    let builder = prepare_viewport_builder(
        app,
        WindowId::Settings,
        "MULTIPLAY :: PREFERENCES",
        Vec2::new(360.0, 240.0),
    );
    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("win_settings"),
        builder,
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                app.dock_manager.set_visible(WindowId::Settings, false);
            }
            handle_satellite_geometry_and_snap(
                app,
                ctx,
                WindowId::Settings,
                Vec2::new(460.0, 400.0),
                false,
            );
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(14, 18, 22))
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| {
                    let mut open_skin = false;
                    SettingsGuiPanel::show(
                        ui,
                        &mut app.settings_tab,
                        &mut app.config,
                        &mut app.theme,
                        &mut open_skin,
                        &mut app.remote,
                    );
                    if open_skin {
                        app.open_skin_dialog();
                    }
                });
        },
    );
}

fn render_dj_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if !app.dock_manager.is_visible(WindowId::DjConsole) {
        app.initialized_viewports.remove(&WindowId::DjConsole);
        app.was_dragging.remove(&WindowId::DjConsole);
        return;
    }
    let builder = prepare_viewport_builder(
        app,
        WindowId::DjConsole,
        "WINAMP :: DJ CONSOLE",
        Vec2::new(380.0, 160.0),
    );
    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("win_dj"),
        builder,
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                app.dock_manager.set_visible(WindowId::DjConsole, false);
            }
            handle_satellite_geometry_and_snap(
                app,
                ctx,
                WindowId::DjConsole,
                Vec2::new(480.0, 220.0),
                true,
            );
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(14, 18, 22))
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| {
                    let selected_track = app.playlist.get(app.selected_track_idx).cloned();
                    DjConsolePanel::show(
                        ui,
                        &mut app.dj_state,
                        selected_track.as_ref(),
                        &app.audio_driver,
                    );
                });
        },
    );
}
