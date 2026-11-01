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
            let current = self
                .audio_driver
                .current_track
                .lock()
                .ok()
                .and_then(|t| t.clone());
            if let Some(t) = current {
                self.dj_played.insert(t.id);
            }
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

    /// Opens (or closes) the DJ headphone device when the setting changed. The engine renders
    /// at the main device's rate, so the headphone device must run at the same rate.
    pub fn sync_dj_headphones(&mut self) {
        let wanted = self.config.audio.dj_headphones_device.clone();
        if wanted.as_deref() == self.dj_phones.as_ref().map(|p| p.0.as_str()) {
            return;
        }
        self.audio_driver.dj.set_phones(None);
        self.dj_phones = None;
        self.dj_phones_error = None;
        let Some(name) = wanted else {
            return;
        };
        let opened = amp_adapter_cpal::CpalAudioSink::new_with_device(Some(&name));
        match opened {
            Ok((mut sink, producer)) => {
                match headphones_rate_ok(self.audio_driver.sample_rate(), sink.sample_rate()) {
                    Ok(()) => {
                        use amp_ports::AudioSinkPort;
                        let _ = sink.start();
                        self.audio_driver.dj.set_phones(Some(producer));
                        self.dj_phones = Some((name, sink));
                    }
                    Err(e) => {
                        self.dj_phones_error = Some(e);
                        self.config.audio.dj_headphones_device = None;
                    }
                }
            }
            Err(e) => {
                self.dj_phones_error = Some(format!("Could not open {name}: {e}"));
                self.config.audio.dj_headphones_device = None;
            }
        }
    }

    /// The library database (analysis cache and hot cues), opened on first use.
    pub(crate) fn library(&mut self) -> Option<Arc<RedbStorageAdapter>> {
        if self.library.is_none() {
            let path = self.paths.data_dir.join("library.redb");
            let _ = std::fs::create_dir_all(&self.paths.data_dir);
            self.library = RedbStorageAdapter::new(path).ok().map(Arc::new);
        }
        self.library.clone()
    }
}

/// Whether a headphone device at `phones` Hz can carry the cue mix of an engine at `main` Hz.
pub fn headphones_rate_ok(main: u32, phones: u32) -> Result<(), String> {
    if main == phones {
        Ok(())
    } else {
        Err(format!(
            "This device runs at {phones} Hz but the main output at {main} Hz. \
             Pick a device at {main} Hz (or change it in the system sound settings)."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_headphones_must_match_the_main_rate() {
        assert!(headphones_rate_ok(48_000, 48_000).is_ok());
        let e = headphones_rate_ok(48_000, 44_100).unwrap_err();
        assert!(e.contains("44100") && e.contains("48000"));
    }
}
