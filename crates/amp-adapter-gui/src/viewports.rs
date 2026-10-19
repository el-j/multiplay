//! Satellite windows (equalizer, playlist, settings, DJ console) rendered as frameless native
//! viewports with self-drawn classic chrome and live magnetic docking.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::chrome::{self, classic_viewport, viewport_id};
use crate::dj_panel::DjConsolePanel;
use crate::docking::WindowId;
use crate::equalizer_widget::EqualizerPanel;
use crate::playlist_window::render_playlist_window_content;
use crate::settings_panel::SettingsGuiPanel;
use eframe::egui::{self, Color32, Pos2, Ui, Vec2, ViewportBuilder};

/// Renders all visible satellite modular windows in their own independent native OS viewports.
pub fn render_satellite_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    let eq = Satellite::new(WindowId::Equalizer, "Equalizer");
    eq.show(app, ctx, |app, ui| {
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

    // The playlist keeps OS resizing until the stepped resize grip lands (TASK-198).
    let mut playlist = Satellite::new(WindowId::Playlist, "Playlist");
    playlist.resizable = true;
    playlist.show(app, ctx, render_playlist_window_content);

    // Settings is an overlay: close button only, never snaps or docks (TASK-201).
    let mut settings = Satellite::new(WindowId::Settings, "Preferences");
    settings.overlay = true;
    settings.show(app, ctx, |app, ui| {
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

    let dj = Satellite::new(WindowId::DjConsole, "DJ Console");
    dj.show(app, ctx, |app, ui| {
        let selected_track = app.playlist.get(app.selected_track_idx).cloned();
        DjConsolePanel::show(
            ui,
            &mut app.dj_state,
            selected_track.as_ref(),
            &app.audio_driver,
        );
    });
}

struct Satellite {
    id: WindowId,
    title: &'static str,
    resizable: bool,
    overlay: bool,
}

impl Satellite {
    fn new(id: WindowId, title: &'static str) -> Self {
        Self {
            id,
            title,
            resizable: false,
            overlay: false,
        }
    }

    fn show(
        &self,
        app: &mut MultiplayGuiApp,
        ctx: &egui::Context,
        mut body: impl FnMut(&mut MultiplayGuiApp, &mut Ui),
    ) {
        if !app.dock_manager.is_visible(self.id) {
            app.initialized_viewports.remove(&self.id);
            if app
                .window_drag
                .as_ref()
                .is_some_and(|d| d.window == self.id)
            {
                app.window_drag = None;
            }
            return;
        }
        let builder = self.builder(app);
        ctx.show_viewport_immediate(viewport_id(self.id), builder, |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                app.dock_manager.set_visible(self.id, false);
            }
            chrome::track_geometry(app, ctx, self.id);
            let full = !self.overlay;
            let action = chrome::title_bar(app, ctx, self.id, self.title, full, full);
            chrome::apply_action(app, ctx, self.id, action);
            if self.overlay && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                app.dock_manager.set_visible(self.id, false);
            }
            if app.dock_manager.is_shaded(self.id) {
                return;
            }
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(14, 18, 22))
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| body(app, ui));
        });
    }

    /// Position and size are only given on first spawn; afterwards the app moves the window.
    fn builder(&self, app: &mut MultiplayGuiApp) -> ViewportBuilder {
        let mut b = classic_viewport(self.title, self.resizable).with_taskbar(false);
        if !app.initialized_viewports.contains(&self.id) {
            let r = app.dock_manager.get_rect(self.id);
            b = b
                .with_inner_size(Vec2::new(r.width, r.height))
                .with_position(Pos2::new(r.x, r.y));
            app.initialized_viewports.insert(self.id);
        }
        b
    }
}
