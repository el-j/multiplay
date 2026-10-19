//! The DJ console that replaces the main window's content in DJ mode (TASK-209): two deck
//! strips around a mixer column. Every row has a fixed height and every text truncates, so the
//! console fits at its minimum size and grows with the window.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_session::DjSession;
use crate::responsive::{row_rtl, ROW};
use eframe::egui::{self, Color32, Rect, RichText, Ui, Vec2};

const MIXER_W: f32 = 96.0;
const DECK_COLORS: [Color32; 2] = [
    Color32::from_rgb(0, 200, 255),
    Color32::from_rgb(255, 150, 40),
];

/// Something the user asked for that needs the whole app.
enum Action {
    LoadSelected(usize),
    Leave,
}

/// Draws the console into `ui` (the main window body).
pub fn render_dj_console(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let Some(dj) = app.dj.as_mut() else {
        return;
    };
    dj.poll();
    let area = ui.available_rect_before_wrap();
    let strip_w = ((area.width() - MIXER_W - 8.0) / 2.0).max(60.0);
    let mut actions = Vec::new();
    for deck in 0..2 {
        let x = if deck == 0 {
            area.left()
        } else {
            area.right() - strip_w
        };
        let rect =
            Rect::from_min_size(egui::pos2(x, area.top()), Vec2::new(strip_w, area.height()));
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.set_clip_rect(rect);
            deck_strip(ui, dj, deck, &mut actions);
        });
    }
    let mixer = Rect::from_center_size(
        egui::pos2(area.center().x, area.center().y),
        Vec2::new(MIXER_W, area.height()),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(mixer), |ui| {
        ui.set_clip_rect(mixer);
        mixer_column(ui, dj, &mut actions);
    });
    ui.allocate_rect(area, egui::Sense::hover());
    for a in actions {
        match a {
            Action::LoadSelected(deck) => {
                if let Some(track) = app.playlist.get(app.selected_track_idx).cloned() {
                    if let Some(dj) = app.dj.as_mut() {
                        dj.load(&app.audio_driver, deck, track);
                    }
                }
            }
            Action::Leave => app.toggle_dj_mode(Some(ui.ctx())),
        }
    }
}

fn deck_strip(ui: &mut Ui, dj: &mut DjSession, deck: usize, actions: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
    let color = DECK_COLORS[deck];
    let label = ["A", "B"][deck];
    let title = dj
        .deck(deck)
        .map_or("Empty — load a track".to_string(), |s| {
            s.track.metadata.title.clone()
        });
    row_rtl(ui, |ui| {
        if ui
            .small_button("⏏")
            .on_hover_text("Load the selected playlist track")
            .clicked()
        {
            actions.push(Action::LoadSelected(deck));
        }
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(RichText::new(label).strong().color(color));
            ui.add(
                egui::Label::new(RichText::new(title).color(Color32::from_rgb(220, 230, 240)))
                    .truncate(),
            );
        });
    });
    let s = dj.status().deck(deck);
    let (pos, bpm) = (s.position_secs(), s.bpm());
    let total = dj
        .deck(deck)
        .and_then(|d| d.analysis().map(|a| a.duration_secs))
        .unwrap_or(s.loaded_secs());
    row_rtl(ui, |ui| {
        let tempo = if bpm > 0.0 {
            format!("{bpm:.1} BPM")
        } else {
            "— BPM".into()
        };
        ui.label(RichText::new(tempo).monospace().size(10.0).color(color));
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let text = format!("{} / {}", clock(pos), clock(total));
            ui.add(egui::Label::new(RichText::new(text).monospace().size(10.0)).truncate());
        });
    });
    // The waveform area takes the spare height (TASK-211 draws the waveforms here).
    let wave_h = (ui.available_height() - 2.0 * ROW - 8.0).max(16.0);
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), wave_h),
        egui::Sense::hover(),
    );
    let progress = if total > 0.0 {
        (pos / total).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    ui.painter()
        .rect_filled(rect, 2.0, Color32::from_rgb(8, 10, 14));
    let played = Rect::from_min_size(rect.min, Vec2::new(rect.width() * progress, rect.height()));
    ui.painter()
        .rect_filled(played, 2.0, color.gamma_multiply(0.25));

    let p = dj.params().deck(deck);
    ui.horizontal(|ui| {
        let playing = p.playing();
        if ui
            .selectable_label(playing, if playing { "⏸" } else { "▶" })
            .clicked()
        {
            p.set_playing(!playing);
        }
        let mut sync = p.sync();
        if ui.toggle_value(&mut sync, "SYNC").changed() {
            p.set_sync(sync);
        }
        let mut pfl = p.pfl();
        if ui
            .toggle_value(&mut pfl, "🎧")
            .on_hover_text("Prelisten (headphones)")
            .changed()
        {
            p.set_pfl(pfl);
        }
    });
    let mut pitch = p.pitch_percent();
    ui.spacing_mut().slider_width = (ui.available_width() - 48.0).max(30.0);
    if ui
        .add(
            egui::Slider::new(&mut pitch, -8.0..=8.0)
                .suffix("%")
                .fixed_decimals(1),
        )
        .on_hover_text("Tempo")
        .changed()
    {
        p.set_pitch_percent(pitch);
    }
}

fn mixer_column(ui: &mut Ui, dj: &mut DjSession, actions: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
    ui.vertical_centered(|ui| {
        if ui
            .small_button("Exit DJ")
            .on_hover_text("Back to normal playback (Ctrl+J)")
            .clicked()
        {
            actions.push(Action::Leave);
        }
        let p = dj.params();
        for deck in 0..2 {
            let mut fader = p.deck(deck).fader();
            ui.spacing_mut().slider_width = MIXER_W - 30.0;
            let label = RichText::new(["A", "B"][deck]).color(DECK_COLORS[deck]);
            ui.horizontal(|ui| {
                ui.label(label);
                if ui
                    .add(egui::Slider::new(&mut fader, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    p.deck(deck).set_fader(fader);
                }
            });
        }
        let mut xf = p.crossfader();
        ui.spacing_mut().slider_width = MIXER_W - 8.0;
        if ui
            .add(egui::Slider::new(&mut xf, -1.0..=1.0).show_value(false))
            .on_hover_text("Crossfader")
            .changed()
        {
            p.set_crossfader(xf);
        }
        ui.label(
            RichText::new("A ◀ X ▶ B")
                .size(9.0)
                .color(Color32::from_rgb(140, 150, 165)),
        );
    });
}

fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
