//! UI rendering logic and multi-viewport window management for Multiplay Native Desktop GUI.
//! Each modular component is rendered as an independent OS native application window.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_panel::DjConsolePanel;
use crate::docking::{DockRect, WindowId};
use crate::equalizer_widget::EqualizerPanel;
use crate::main_window::render_main_window_content;
use crate::playlist_window::render_playlist_window_content;
use crate::settings_panel::SettingsGuiPanel;
use eframe::egui::{self, Color32, Pos2, Vec2, ViewportBuilder, ViewportCommand, ViewportId};
use std::time::Duration;

impl eframe::App for MultiplayGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous smooth repaint for 60 FPS real-time visualizer animation
        ctx.request_repaint_after(Duration::from_millis(16));

        // 1. Automatic Gapless Track Advancement
        self.check_gapless_advancement();

        // 2. OS-Native Drag-and-Drop Ingestion (Files & Folders on Main Player)
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped_files {
            if let Some(path) = f.path {
                self.ingest_path(&path);
            }
        }

        // 3. Track Root Window (Main Player) Desktop Position & Propagate Group Movement
        if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
            let old_main = self.dock_manager.get_rect(WindowId::Main);
            let dx = outer.min.x - old_main.x;
            let dy = outer.min.y - old_main.y;
            self.dock_manager.set_rect(
                WindowId::Main,
                DockRect::new(outer.min.x, outer.min.y, outer.width(), outer.height()),
            );
            if dx.abs() > 1.0 || dy.abs() > 1.0 {
                self.dock_manager.move_window_group(WindowId::Main, dx, dy);
                self.sync_child_viewport_positions(ctx);
            }
        }

        // 4. Render Main Player Content in Root Viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(14, 18, 22)).inner_margin(8.0))
            .show(ctx, |ui| {
                render_main_window_content(self, ui);
            });

        // 5. Render Modular Satellite OS Native Windows
        render_satellite_windows(self, ctx);
    }
}

/// Renders all visible satellite modular windows in their own independent native OS viewports.
pub fn render_satellite_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    // 1. EQUALIZER OS WINDOW
    render_equalizer_viewport(app, ctx);

    // 2. PLAYLIST OS WINDOW
    render_playlist_viewport(app, ctx);

    // 3. PREFERENCES OS WINDOW
    render_settings_viewport(app, ctx);

    // 4. DJ CONSOLE OS WINDOW
    render_dj_viewport(app, ctx);
}

/// Backwards-compatible alias for modular window rendering.
#[inline]
pub fn render_modular_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    render_satellite_windows(app, ctx);
}

fn render_equalizer_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if app.dock_manager.is_visible(WindowId::Equalizer) {
        let eq_rect = app.dock_manager.get_rect(WindowId::Equalizer);
        let mut builder = ViewportBuilder::default()
            .with_title("WINAMP :: EQUALIZER")
            .with_inner_size(Vec2::new(eq_rect.width, eq_rect.height))
            .with_min_inner_size(Vec2::new(380.0, 150.0))
            .with_resizable(true);

        if !app.initialized_viewports.contains(&WindowId::Equalizer) {
            builder = builder.with_position(Pos2::new(eq_rect.x, eq_rect.y));
            app.initialized_viewports.insert(WindowId::Equalizer);
        }

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("win_eq"),
            builder,
            |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    app.dock_manager.set_visible(WindowId::Equalizer, false);
                }
                if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                    let current = DockRect::new(outer.min.x, outer.min.y, outer.width(), outer.height());
                    let (snapped, dock_info) = app.dock_manager.compute_snap(WindowId::Equalizer, current);
                    if (snapped.x - current.x).abs() > 1.0 || (snapped.y - current.y).abs() > 1.0 {
                        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(snapped.x, snapped.y)));
                    }
                    app.dock_manager.set_rect(WindowId::Equalizer, snapped);
                    if let Some((parent, edge)) = dock_info {
                        app.dock_manager.dock(WindowId::Equalizer, parent, edge);
                    } else {
                        app.dock_manager.undock(WindowId::Equalizer);
                    }
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::from_rgb(14, 18, 22)).inner_margin(8.0))
                    .show(ctx, |ui| {
                        let prev_preamp = app.eq_preamp;
                        let prev_gains = app.eq_gains;
                        let prev_enabled = app.eq_enabled;

                        EqualizerPanel::new(&mut app.eq_preamp, &mut app.eq_gains, &mut app.eq_enabled).ui(ui);

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
    } else {
        app.initialized_viewports.remove(&WindowId::Equalizer);
    }
}

fn render_playlist_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if app.dock_manager.is_visible(WindowId::Playlist) {
        let pl_rect = app.dock_manager.get_rect(WindowId::Playlist);
        let mut builder = ViewportBuilder::default()
            .with_title("WINAMP :: PLAYLIST")
            .with_inner_size(Vec2::new(pl_rect.width, pl_rect.height))
            .with_min_inner_size(Vec2::new(380.0, 180.0))
            .with_resizable(true);

        if !app.initialized_viewports.contains(&WindowId::Playlist) {
            builder = builder.with_position(Pos2::new(pl_rect.x, pl_rect.y));
            app.initialized_viewports.insert(WindowId::Playlist);
        }

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("win_playlist"),
            builder,
            |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    app.dock_manager.set_visible(WindowId::Playlist, false);
                }
                if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                    let current = DockRect::new(outer.min.x, outer.min.y, outer.width(), outer.height());
                    let (snapped, dock_info) = app.dock_manager.compute_snap(WindowId::Playlist, current);
                    if (snapped.x - current.x).abs() > 1.0 || (snapped.y - current.y).abs() > 1.0 {
                        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(snapped.x, snapped.y)));
                    }
                    app.dock_manager.set_rect(WindowId::Playlist, snapped);
                    if let Some((parent, edge)) = dock_info {
                        app.dock_manager.dock(WindowId::Playlist, parent, edge);
                    } else {
                        app.dock_manager.undock(WindowId::Playlist);
                    }
                }
                // Native Drag-and-Drop files onto Playlist Window
                let dropped = ctx.input(|i| i.raw.dropped_files.clone());
                for f in dropped {
                    if let Some(p) = f.path {
                        app.ingest_path(&p);
                    }
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::from_rgb(14, 18, 22)).inner_margin(8.0))
                    .show(ctx, |ui| {
                        render_playlist_window_content(app, ui);
                    });
            },
        );
    } else {
        app.initialized_viewports.remove(&WindowId::Playlist);
    }
}

fn render_settings_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if app.dock_manager.is_visible(WindowId::Settings) {
        let set_rect = app.dock_manager.get_rect(WindowId::Settings);
        let mut builder = ViewportBuilder::default()
            .with_title("MULTIPLAY :: PREFERENCES")
            .with_inner_size(Vec2::new(set_rect.width, set_rect.height))
            .with_min_inner_size(Vec2::new(360.0, 240.0))
            .with_resizable(true);

        if !app.initialized_viewports.contains(&WindowId::Settings) {
            builder = builder.with_position(Pos2::new(set_rect.x, set_rect.y));
            app.initialized_viewports.insert(WindowId::Settings);
        }

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("win_settings"),
            builder,
            |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    app.dock_manager.set_visible(WindowId::Settings, false);
                }
                if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                    app.dock_manager.set_rect(
                        WindowId::Settings,
                        DockRect::new(outer.min.x, outer.min.y, outer.width(), outer.height()),
                    );
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::from_rgb(14, 18, 22)).inner_margin(8.0))
                    .show(ctx, |ui| {
                        SettingsGuiPanel::show(ui, &mut app.settings_tab, &mut app.config);
                    });
            },
        );
    } else {
        app.initialized_viewports.remove(&WindowId::Settings);
    }
}

fn render_dj_viewport(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if app.dock_manager.is_visible(WindowId::DjConsole) {
        let dj_rect = app.dock_manager.get_rect(WindowId::DjConsole);
        let mut builder = ViewportBuilder::default()
            .with_title("WINAMP :: DJ CONSOLE")
            .with_inner_size(Vec2::new(dj_rect.width, dj_rect.height))
            .with_min_inner_size(Vec2::new(380.0, 160.0))
            .with_resizable(true);

        if !app.initialized_viewports.contains(&WindowId::DjConsole) {
            builder = builder.with_position(Pos2::new(dj_rect.x, dj_rect.y));
            app.initialized_viewports.insert(WindowId::DjConsole);
        }

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("win_dj"),
            builder,
            |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    app.dock_manager.set_visible(WindowId::DjConsole, false);
                }
                if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                    app.dock_manager.set_rect(
                        WindowId::DjConsole,
                        DockRect::new(outer.min.x, outer.min.y, outer.width(), outer.height()),
                    );
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::from_rgb(14, 18, 22)).inner_margin(8.0))
                    .show(ctx, |ui| {
                        DjConsolePanel::show(ui, &mut app.dj_state);
                    });
            },
        );
    } else {
        app.initialized_viewports.remove(&WindowId::DjConsole);
    }
}
