//! Satellite windows (equalizer, playlist, settings, DJ console) rendered as frameless native
//! viewports with self-drawn classic chrome and live magnetic docking.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::chrome::resize::min_inner_size;
use crate::chrome::{self, classic_viewport, viewport_id};
use crate::dj_panel::DjConsolePanel;
use crate::docking::{separate_from, DockRect, WindowId};
use crate::equalizer_widget::EqualizerPanel;
use crate::main_window::render_main_window_content;
use crate::playlist_window::render_playlist_window_content;
use crate::settings_panel::SettingsGuiPanel;
use eframe::egui::{self, Color32, Pos2, Ui, Vec2, ViewportBuilder};

/// Renders all visible satellite modular windows in their own independent native OS viewports.
pub fn render_satellite_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    Satellite::new(WindowId::Equalizer, "Equalizer").show(app, ctx);
    Satellite::new(WindowId::Playlist, "Playlist").show(app, ctx);
    // Settings is an overlay: close button only, never snaps or docks (TASK-201).
    let mut settings = Satellite::new(WindowId::Settings, "Preferences");
    settings.overlay = true;
    settings.show(app, ctx);
    Satellite::new(WindowId::DjConsole, "DJ Console").show(app, ctx);
}

/// Renders `window`'s content under its title strip, grows the window if anything still did
/// not fit, and adds the resize handles.
pub fn show_body(app: &mut MultiplayGuiApp, ctx: &egui::Context, window: WindowId) {
    chrome::zoom::shortcut(ctx);
    if ctx.input_mut(|i| i.consume_shortcut(&chrome::overlay::SETTINGS)) {
        app.dock_manager.toggle_visible(WindowId::Settings);
    }
    if app.dock_manager.is_shaded(window) {
        return;
    }
    let mut spill = Vec2::ZERO;
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(Color32::from_rgb(14, 18, 22))
                .inner_margin(8.0),
        )
        .show(ctx, |ui| {
            let avail = ui.max_rect();
            window_body(app, window, ui);
            let used = ui.min_rect();
            spill = Vec2::new(
                (used.right() - avail.right()).max(0.0),
                (used.bottom() - avail.bottom()).max(0.0),
            );
        });
    chrome::handles::grow_to_fit(app, ctx, window, spill);
    chrome::handles::resize_handles(app, ctx, window);
}

/// Rect for a window that is being shown: at least its minimum size, and moved clear of every
/// other visible window so it never opens on top of one.
fn place_on_show(app: &mut MultiplayGuiApp, window: WindowId, screens: &[DockRect]) -> DockRect {
    let min = min_inner_size(window);
    let mut r = app.dock_manager.get_rect(window);
    r.width = r.width.max(min.x);
    r.height = r.height.max(min.y);
    if window == WindowId::Settings {
        let main = app.dock_manager.get_rect(WindowId::Main);
        r = chrome::overlay::centered(r.width, r.height, screens.first().copied(), main);
    } else {
        let others: Vec<DockRect> = app
            .dock_manager
            .dockable_windows()
            .into_iter()
            .filter(|(id, _)| *id != window)
            .map(|(_, r)| r)
            .collect();
        r = separate_from(r, &others);
    }
    app.dock_manager.set_rect(window, r);
    r
}

/// Content of `window` below its title strip. Shared by the real viewports and the offscreen
/// responsive-layout tests, so both exercise exactly the same layout code.
pub fn window_body(app: &mut MultiplayGuiApp, window: WindowId, ui: &mut Ui) {
    // Baseline guarantee: text never grows past its space; it ends in an ellipsis instead.
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
    match window {
        WindowId::Main => render_main_window_content(app, ui),
        WindowId::Equalizer => {
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
        }
        WindowId::Playlist => render_playlist_window_content(app, ui),
        WindowId::Settings => {
            let mut open_skin = false;
            let done = SettingsGuiPanel::show(
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
            if done {
                app.dock_manager.set_visible(WindowId::Settings, false);
            }
        }
        WindowId::DjConsole => {
            // Legacy console scrolls until the responsive DJ console replaces it (PLAN-055).
            let selected_track = app.playlist.get(app.selected_track_idx).cloned();
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    DjConsolePanel::show(
                        ui,
                        &mut app.dj_state,
                        selected_track.as_ref(),
                        &app.audio_driver,
                    );
                });
        }
    }
}

struct Satellite {
    id: WindowId,
    title: &'static str,
    overlay: bool,
}

impl Satellite {
    fn new(id: WindowId, title: &'static str) -> Self {
        Self {
            id,
            title,
            overlay: false,
        }
    }

    fn show(&self, app: &mut MultiplayGuiApp, ctx: &egui::Context) {
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
        let builder = self.builder(app, &chrome::screens(ctx));
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
            show_body(app, ctx, self.id);
        });
    }

    /// Position and size are only given on first spawn; afterwards the app moves the window.
    fn builder(&self, app: &mut MultiplayGuiApp, screens: &[DockRect]) -> ViewportBuilder {
        let mut b = classic_viewport(self.title, false).with_taskbar(false);
        if !app.initialized_viewports.contains(&self.id) {
            let r = place_on_show(app, self.id, screens);
            b = b
                .with_inner_size(Vec2::new(r.width, r.height))
                .with_position(Pos2::new(r.x, r.y));
            app.initialized_viewports.insert(self.id);
        }
        b
    }
}
