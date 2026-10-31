//! Turns playback errors from the engine into a short notice for the user, so a track that
//! cannot be played (a broken provider, a missing file) says why instead of being skipped silently.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use amp_core_engine::events::PlayerEvent;
use tokio::sync::broadcast::error::TryRecvError;

impl MultiplayGuiApp {
    /// Reads the engine's events since the last frame and updates [`Self::playback_notice`].
    pub fn poll_playback_events(&mut self) {
        let Some(events) = self.playback_events.as_mut() else {
            return;
        };
        let mut seen = Vec::new();
        loop {
            match events.try_recv() {
                Ok(event) => seen.push(event),
                Err(TryRecvError::Lagged(_)) => continue,
                Err(TryRecvError::Empty | TryRecvError::Closed) => break,
            }
        }
        for event in seen {
            match event {
                PlayerEvent::TrackStarted(_) => self.playback_notice = None,
                PlayerEvent::Error {
                    source, message, ..
                } => {
                    let title = source
                        .and_then(|s| self.playlist.iter().find(|t| t.source == s))
                        .map(|t| format!("“{}”", t.metadata.title))
                        .unwrap_or_else(|| "this track".into());
                    self.playback_notice = Some(format!("Couldn't play {title}: {message}"));
                }
                _ => {}
            }
        }
    }
}

/// Shows the playback notice, if any, as one line the user can dismiss.
pub fn notice_line(app: &mut MultiplayGuiApp, ui: &mut eframe::egui::Ui) {
    use eframe::egui::{Color32, Label, RichText};
    let Some(notice) = app.playback_notice.clone() else {
        return;
    };
    crate::responsive::row_rtl(ui, |ui| {
        if ui.small_button("🗙").on_hover_text("Dismiss").clicked() {
            app.playback_notice = None;
        }
        ui.add(
            Label::new(
                RichText::new(format!("⚠ {notice}"))
                    .size(10.0)
                    .color(Color32::from_rgb(255, 170, 80)),
            )
            .truncate(),
        )
        .on_hover_text(&notice);
    });
}
