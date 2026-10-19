//! The DJ console that replaces the main window's content in DJ mode (TASK-209): two deck
//! strips around a mixer column. Every row has a fixed height and every text truncates, so the
//! console fits at its minimum size and grows with the window.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_session::{DjSession, PlaylistDeckSource};
use crate::dj_waveform::{overview, paint_lane, paint_phase_meter, LaneView, Marks};
use crate::responsive::{row_rtl, ROW};
use amp_core_engine::dj::engine::sync::{phase_error, DeckBeat};
use eframe::egui::{self, Color32, Rect, RichText, Ui, Vec2};

const MIXER_W: f32 = 96.0;
const DECK_COLORS: [Color32; 2] = [
    Color32::from_rgb(0, 200, 255),
    Color32::from_rgb(255, 150, 40),
];

/// Something the user asked for that needs the whole app.
enum Action {
    LoadNext(usize),
    ToggleAuto,
    Leave,
}

/// Draws the console into `ui` (the main window body).
pub fn render_dj_console(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let Some(dj) = app.dj.as_mut() else {
        return;
    };
    dj.poll();
    let full = ui.available_rect_before_wrap();
    let lanes_h = (full.height() * 0.3).clamp(34.0, 140.0);
    let lanes = Rect::from_min_size(full.min, Vec2::new(full.width(), lanes_h));
    paint_lanes(ui, dj, lanes);
    let area = Rect::from_min_max(egui::pos2(full.left(), lanes.bottom() + 4.0), full.max);
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
    ui.allocate_rect(full, egui::Sense::hover());
    let Some(mut dj) = app.dj.take() else {
        return;
    };
    let resolvers = app
        .audio_driver
        .resolvers
        .lock()
        .map(|r| r.clone())
        .unwrap_or_default();
    let mut source = PlaylistDeckSource::new(
        &app.playlist,
        &app.deck_assign,
        &mut app.dj_played,
        resolvers,
        app.library.clone(),
    );
    let mut leave = false;
    for a in actions {
        match a {
            Action::LoadNext(deck) => {
                // The deck's next assigned track, else the selected row.
                let pick = source.peek(deck).or(Some(app.selected_track_idx));
                if let Some(track) = pick.and_then(|i| app.playlist.get(i)).cloned() {
                    source.played.insert(track.id);
                    dj.load(&app.audio_driver, deck, track);
                }
            }
            Action::ToggleAuto if dj.automix_on() => dj.stop_automix(),
            Action::ToggleAuto => dj.start_automix(&mut source),
            Action::Leave => leave = true,
        }
    }
    dj.tick_automix(&mut source);
    app.dj = Some(dj);
    if leave {
        app.toggle_dj_mode(Some(ui.ctx()));
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
            .on_hover_text("Load the next track assigned to this deck (or the selected one)")
            .clicked()
        {
            actions.push(Action::LoadNext(deck));
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
        let pitch = dj.params().deck(deck).pitch_percent();
        ui.label(
            RichText::new(format!("{pitch:+.1}%"))
                .monospace()
                .size(10.0),
        );
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let text = format!("{} / {}", clock(pos), clock(total));
            ui.add(egui::Label::new(RichText::new(text).monospace().size(10.0)).truncate());
        });
    });
    // The overview takes the spare height; click it to jump.
    let wave_h = (ui.available_height() - 2.0 * ROW - 8.0).max(16.0);
    let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), wave_h));
    let cues: [Option<f64>; 8] = std::array::from_fn(|i| s.hot_cue(i));
    let marks = Marks {
        cues: &cues,
        loop_range: s.loop_range(),
        loop_active: s.loop_active(),
    };
    let analysis = dj.deck(deck).and_then(|d| d.analysis().cloned());
    if let Some(t) = overview(ui, rect, analysis.as_ref(), pos, total, color, marks) {
        dj.control().seek(deck, t);
    }

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
        ui.horizontal(|ui| {
            if ui
                .small_button("Exit")
                .on_hover_text("Back to normal playback (Ctrl+J)")
                .clicked()
            {
                actions.push(Action::Leave);
            }
            let auto = dj.automix_on();
            if ui
                .selectable_label(auto, RichText::new("AUTO").size(10.0))
                .on_hover_text("Auto mix through the assigned playlist")
                .clicked()
            {
                actions.push(Action::ToggleAuto);
            }
        });
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

/// The stacked detail lanes (deck A above deck B) with the phase meter between them.
fn paint_lanes(ui: &Ui, dj: &DjSession, rect: Rect) {
    let meter_h = 4.0;
    let lane_h = (rect.height() - meter_h - 2.0) / 2.0;
    let painter = ui.painter_at(rect);
    let mut beats = [None, None];
    for deck in 0..2 {
        let y = if deck == 0 {
            rect.top()
        } else {
            rect.bottom() - lane_h
        };
        let lane = Rect::from_min_size(egui::pos2(rect.left(), y), Vec2::new(rect.width(), lane_h));
        let s = dj.status().deck(deck);
        let analysis = dj.deck(deck).and_then(|d| d.analysis());
        match analysis {
            Some(a) => {
                let bpm = a.bpm.unwrap_or(0.0);
                let speed = if bpm > 0.0 && s.bpm() > 0.0 {
                    s.bpm() / bpm
                } else {
                    f64::from(1.0 + dj.params().deck(deck).pitch_percent() / 100.0)
                };
                let view = LaneView {
                    analysis: a,
                    position: s.position_secs(),
                    speed,
                    color: DECK_COLORS[deck],
                };
                paint_lane(&painter, lane, &view);
                if bpm > 0.0 && dj.params().deck(deck).playing() {
                    beats[deck] = Some(DeckBeat {
                        bpm,
                        anchor: a.first_downbeat_secs,
                        position: s.position_secs(),
                        speed,
                    });
                }
            }
            None => {
                painter.rect_filled(lane, 2.0, Color32::from_rgb(8, 10, 14));
                let text = if dj.deck(deck).is_some() {
                    "analysing…"
                } else {
                    ""
                };
                painter.text(
                    lane.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(110),
                );
            }
        }
    }
    let meter = Rect::from_center_size(rect.center(), Vec2::new(rect.width() * 0.3, meter_h));
    let error = match beats {
        [Some(a), Some(b)] => Some(phase_error(&a, &b)),
        _ => None,
    };
    paint_phase_meter(&painter, meter, error);
}
