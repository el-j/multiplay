//! UI rendering logic and window management for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_panel::DjConsolePanel;
use crate::docking::{DockRect, WindowId};
use crate::equalizer_widget::EqualizerPanel;
use crate::main_window::render_main_window_content;
use crate::playlist_window::render_playlist_window_content;
use crate::settings_panel::SettingsGuiPanel;
use eframe::egui::{self, Color32, Pos2, Vec2};
use std::time::Duration;

impl eframe::App for MultiplayGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous smooth repaint for 60 FPS real-time visualizer animation
        ctx.request_repaint_after(Duration::from_millis(16));

        // 1. Automatic Gapless Track Advancement
        self.check_gapless_advancement();

        // 2. OS-Native Drag-and-Drop Ingestion (Files & Folders)
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped_files {
            if let Some(path) = f.path {
                self.ingest_path(&path);
            }
        }

        // 3. Render Desktop Background
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(10, 14, 18)))
            .show(ctx, |_ui| {});

        // 4. Modular Windows Rendering & Snapping
        render_modular_windows(self, ctx);
    }
}

/// Renders all visible modular windows and calculates magnetic docking.
pub fn render_modular_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    // 1. MAIN PLAYER WINDOW
    let main_rect = app.dock_manager.get_rect(WindowId::Main);
    let mut main_open = app.dock_manager.is_visible(WindowId::Main);
    let mut main_pos = Pos2::new(main_rect.x, main_rect.y);

    let main_resp = egui::Window::new("WINAMP :: MAIN PLAYER")
        .id(egui::Id::new("win_main"))
        .open(&mut main_open)
        .current_pos(main_pos)
        .default_size(Vec2::new(main_rect.width, main_rect.height))
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            render_main_window_content(app, ui);
        });

    if let Some(resp) = main_resp {
        if resp.response.dragged() {
            let delta = resp.response.drag_delta();
            app.dock_manager.move_window_group(WindowId::Main, delta.x, delta.y);
        } else {
            main_pos = resp.response.rect.min;
            let current = DockRect::new(main_pos.x, main_pos.y, resp.response.rect.width(), resp.response.rect.height());
            let (snapped, _) = app.dock_manager.compute_snap(WindowId::Main, current);
            app.dock_manager.set_rect(WindowId::Main, snapped);
        }
    }

    // 2. EQUALIZER WINDOW
    if app.dock_manager.is_visible(WindowId::Equalizer) {
        let eq_rect = app.dock_manager.get_rect(WindowId::Equalizer);
        let mut eq_open = true;
        let mut eq_pos = Pos2::new(eq_rect.x, eq_rect.y);

        let eq_resp = egui::Window::new("WINAMP :: EQUALIZER")
            .id(egui::Id::new("win_eq"))
            .open(&mut eq_open)
            .current_pos(eq_pos)
            .default_size(Vec2::new(eq_rect.width, eq_rect.height))
            .resizable(true)
            .collapsible(true)
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

        if !eq_open {
            app.dock_manager.set_visible(WindowId::Equalizer, false);
        }

        if let Some(resp) = eq_resp {
            if resp.response.dragged() {
                let delta = resp.response.drag_delta();
                app.dock_manager.move_window_group(WindowId::Equalizer, delta.x, delta.y);
            } else {
                eq_pos = resp.response.rect.min;
                let current = DockRect::new(eq_pos.x, eq_pos.y, resp.response.rect.width(), resp.response.rect.height());
                let (snapped, dock_info) = app.dock_manager.compute_snap(WindowId::Equalizer, current);
                app.dock_manager.set_rect(WindowId::Equalizer, snapped);
                if let Some((parent, edge)) = dock_info {
                    app.dock_manager.dock(WindowId::Equalizer, parent, edge);
                }
            }
        }
    }

    // 3. PLAYLIST WINDOW
    if app.dock_manager.is_visible(WindowId::Playlist) {
        let pl_rect = app.dock_manager.get_rect(WindowId::Playlist);
        let mut pl_open = true;
        let mut pl_pos = Pos2::new(pl_rect.x, pl_rect.y);

        let pl_resp = egui::Window::new("WINAMP :: PLAYLIST")
            .id(egui::Id::new("win_playlist"))
            .open(&mut pl_open)
            .current_pos(pl_pos)
            .default_size(Vec2::new(pl_rect.width, pl_rect.height))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                render_playlist_window_content(app, ui);
            });

        if !pl_open {
            app.dock_manager.set_visible(WindowId::Playlist, false);
        }

        if let Some(resp) = pl_resp {
            if resp.response.dragged() {
                let delta = resp.response.drag_delta();
                app.dock_manager.move_window_group(WindowId::Playlist, delta.x, delta.y);
            } else {
                pl_pos = resp.response.rect.min;
                let current = DockRect::new(pl_pos.x, pl_pos.y, resp.response.rect.width(), resp.response.rect.height());
                let (snapped, dock_info) = app.dock_manager.compute_snap(WindowId::Playlist, current);
                app.dock_manager.set_rect(WindowId::Playlist, snapped);
                if let Some((parent, edge)) = dock_info {
                    app.dock_manager.dock(WindowId::Playlist, parent, edge);
                }
            }
        }
    }

    // 4. PREFERENCES WINDOW
    if app.dock_manager.is_visible(WindowId::Settings) {
        let set_rect = app.dock_manager.get_rect(WindowId::Settings);
        let mut set_open = true;

        let set_resp = egui::Window::new("MULTIPLAY :: PREFERENCES")
            .id(egui::Id::new("win_settings"))
            .open(&mut set_open)
            .current_pos(Pos2::new(set_rect.x, set_rect.y))
            .default_size(Vec2::new(set_rect.width, set_rect.height))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                SettingsGuiPanel::show(ui, &mut app.settings_tab, &mut app.config);
            });

        if !set_open {
            app.dock_manager.set_visible(WindowId::Settings, false);
        }

        if let Some(resp) = set_resp {
            let pos = resp.response.rect.min;
            app.dock_manager.set_rect(WindowId::Settings, DockRect::new(pos.x, pos.y, resp.response.rect.width(), resp.response.rect.height()));
        }
    }

    // 5. DJ CONSOLE WINDOW
    if app.dock_manager.is_visible(WindowId::DjConsole) {
        let dj_rect = app.dock_manager.get_rect(WindowId::DjConsole);
        let mut dj_open = true;

        let dj_resp = egui::Window::new("WINAMP :: DJ CONSOLE")
            .id(egui::Id::new("win_dj"))
            .open(&mut dj_open)
            .current_pos(Pos2::new(dj_rect.x, dj_rect.y))
            .default_size(Vec2::new(dj_rect.width, dj_rect.height))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                DjConsolePanel::show(ui, &mut app.dj_state);
            });

        if !dj_open {
            app.dock_manager.set_visible(WindowId::DjConsole, false);
        }

        if let Some(resp) = dj_resp {
            let pos = resp.response.rect.min;
            app.dock_manager.set_rect(WindowId::DjConsole, DockRect::new(pos.x, pos.y, resp.response.rect.width(), resp.response.rect.height()));
        }
    }
}
