//! Turning DJ mode on and off from the app (TASK-209): the main window becomes the DJ console
//! (and back), the playing track continues as deck A, and the docked windows follow the size
//! change.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_session::DjSession;
use crate::docking::WindowId;
use amp_adapter_storage::RedbStorageAdapter;
use eframe::egui::{self, Vec2};
use std::sync::Arc;

/// Console size when DJ mode opens: two deck strips and a mixer column.
pub const CONSOLE_SIZE: Vec2 = Vec2::new(550.0, 232.0);
/// Smallest console size at which every DJ control still fits.
pub const CONSOLE_MIN: Vec2 = Vec2::new(520.0, 232.0);

impl MultiplayGuiApp {
    /// True while DJ mode is on.
    pub fn dj_active(&self) -> bool {
        self.dj.as_ref().is_some_and(DjSession::is_active)
    }

    /// Smallest size of `window` right now (the main window is larger as the DJ console).
    pub fn min_size(&self, window: WindowId) -> Vec2 {
        let base = crate::chrome::resize::min_inner_size(window);
        if window == WindowId::Main && self.dj_active() {
            base.max(CONSOLE_MIN)
        } else {
            base
        }
    }

    /// Switches DJ mode on or off. With a `ctx`, the main window is resized to the console and
    /// back (the docked windows follow).
    pub fn toggle_dj_mode(&mut self, ctx: Option<&egui::Context>) {
        if self.dj.is_none() {
            let library = self.library();
            self.dj = Some(DjSession::new(&self.audio_driver, library));
        }
        let Some(dj) = self.dj.as_mut() else {
            return;
        };
        let entering = !dj.is_active();
        if entering {
            dj.enter(&self.audio_driver);
        } else {
            dj.leave(&self.audio_driver);
        }
        let Some(ctx) = ctx else {
            return;
        };
        let rect = self.dock_manager.get_rect(WindowId::Main);
        let size = if entering {
            self.normal_main_size = Some(Vec2::new(rect.width, rect.height));
            CONSOLE_SIZE.max(Vec2::new(rect.width, rect.height))
        } else {
            self.normal_main_size
                .take()
                .unwrap_or_else(|| crate::chrome::resize::min_inner_size(WindowId::Main))
        };
        crate::chrome::handles::resize_window(self, ctx, WindowId::Main, size.x, size.y);
    }

    /// The library database (analysis cache and hot cues), opened on first use.
    fn library(&mut self) -> Option<Arc<RedbStorageAdapter>> {
        if self.library.is_none() {
            let path = self.paths.data_dir.join("library.redb");
            let _ = std::fs::create_dir_all(&self.paths.data_dir);
            self.library = RedbStorageAdapter::new(path).ok().map(Arc::new);
        }
        self.library.clone()
    }
}
