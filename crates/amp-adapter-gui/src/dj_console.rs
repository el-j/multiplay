//! The DJ console that replaces the main window's content in DJ mode (TASK-209/211/212): the
//! stacked waveforms on top, two deck strips around a mixer column below, the keyboard map,
//! the cheat sheet and the first-run hint. Every row has a fixed height and every text
//! truncates, so the console fits at its minimum size and grows with the window.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_controls::{
    describe, key_label, keymap, press, release, Binding, ConsoleState, DjAction,
};
use crate::dj_session::{DjSession, PlaylistDeckSource};
use crate::dj_strip::{deck_strip, mixer_column, Panel, UiOut, DECK_COLORS};
use crate::dj_waveform::{paint_lane, paint_phase_meter, LaneView};
use amp_core_engine::dj::engine::sync::{phase_error, DeckBeat};
use eframe::egui::{self, Color32, Rect, RichText, Ui, Vec2};
use std::time::{Duration, Instant};

const MIXER_W: f32 = 104.0;

/// Draws the console into `ui` (the main window body).
pub fn render_dj_console(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let Some(mut dj) = app.dj.take() else {
        return;
    };
    dj.poll();
    let now = Instant::now();
    let mut st = std::mem::take(&mut app.dj_console);
    st.rendered.clear();
    let mut out = Vec::new();
    keyboard(ui, &mut dj, &mut st, &mut out, now);

    let full = ui.available_rect_before_wrap();
    let lanes_h = (full.height() * 0.24).clamp(28.0, 140.0);
    let lanes = Rect::from_min_size(full.min, Vec2::new(full.width(), lanes_h));
    paint_lanes(ui, &dj, lanes);
    let area = Rect::from_min_max(egui::pos2(full.left(), lanes.bottom() + 3.0), full.max);
    let strip_w = ((area.width() - MIXER_W - 8.0) / 2.0).max(60.0);
    {
        let mut panel = Panel {
            dj: &mut dj,
            st: &mut st,
            out: &mut out,
            now,
        };
        for deck in 0..2 {
            let x = if deck == 0 {
                area.left()
            } else {
                area.right() - strip_w
            };
            let rect =
                Rect::from_min_size(egui::pos2(x, area.top()), Vec2::new(strip_w, area.height()));
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.set_clip_rect(rect);
                deck_strip(&mut panel, ui, deck);
            });
        }
        let mixer = Rect::from_center_size(area.center(), Vec2::new(MIXER_W, area.height()));
        ui.scope_builder(egui::UiBuilder::new().max_rect(mixer), |ui| {
            ui.set_clip_rect(mixer);
            mixer_column(&mut panel, ui, MIXER_W);
        });
    }
    ui.allocate_rect(full, egui::Sense::hover());
    if !app.dj_hint_done && first_run_hint(ui, lanes) {
        app.dj_hint_done = true;
        app.save_session();
    }
    if st.cheat_sheet && cheat_sheet(ui, full) {
        st.cheat_sheet = false;
    }
    message(ui, &mut st, lanes, now);

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
    for o in out {
        match o {
            UiOut::LoadNext(deck) => {
                // The deck's next assigned track, else the selected row.
                let pick = source.peek(deck).or(Some(app.selected_track_idx));
                if let Some(track) = pick.and_then(|i| app.playlist.get(i)).cloned() {
                    source.played.insert(track.id);
                    dj.load(&app.audio_driver, deck, track);
                }
            }
            UiOut::ToggleAuto if dj.automix_on() => dj.stop_automix(),
            UiOut::ToggleAuto => dj.start_automix(&mut source),
            UiOut::Leave => leave = true,
        }
    }
    dj.tick_automix(&mut source);
    app.dj = Some(dj);
    app.dj_console = st;
    if leave {
        app.toggle_dj_mode(Some(ui.ctx()));
    }
    // Waveforms, meters and the playhead move continuously.
    ui.ctx().request_repaint_after(Duration::from_millis(16));
}

/// Runs the keyboard map (not while a text field has focus).
fn keyboard(
    ui: &Ui,
    dj: &mut DjSession,
    st: &mut ConsoleState,
    out: &mut Vec<UiOut>,
    now: Instant,
) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let map = keymap();
    let events = ui.input(|i| i.events.clone());
    for e in events {
        let egui::Event::Key {
            key,
            pressed,
            repeat,
            modifiers,
            ..
        } = e
        else {
            continue;
        };
        if modifiers.command || modifiers.ctrl {
            continue;
        }
        let Some(bind) = find(&map, key, modifiers) else {
            continue;
        };
        if pressed && !repeat {
            if press(dj, st, bind.action, modifiers.shift, now) {
                out.push(UiOut::ToggleAuto);
            }
        } else if !pressed {
            release(dj, st, bind.action);
        }
    }
}

/// The binding for `key` with `m`. Pitch keys also take Shift for a bigger step.
fn find(map: &[Binding], key: egui::Key, m: egui::Modifiers) -> Option<Binding> {
    let same_key = |b: &&Binding| b.key == key && b.modifiers.alt == m.alt;
    map.iter()
        .filter(same_key)
        .find(|b| b.modifiers.shift == m.shift)
        .or_else(|| {
            map.iter()
                .filter(same_key)
                .find(|b| matches!(b.action, DjAction::PitchDown(_) | DjAction::PitchUp(_)))
        })
        .copied()
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
        match dj.deck(deck).and_then(|d| d.analysis()) {
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

/// The three-step first-run hint over the waveforms. Returns true when dismissed.
fn first_run_hint(ui: &mut Ui, over: Rect) -> bool {
    let mut done = false;
    egui::Area::new(egui::Id::unique("dj_first_run_hint"))
        .fixed_pos(over.min)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(over.width() - 12.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.label(RichText::new("Mixing in three steps").strong());
                ui.label("1  Assign tracks in the playlist: click a row's badge or press [ ] \\.");
                ui.label("2  Load deck B (⏏), listen with 🎧; SYNC lines up the beats.");
                ui.label("3  Slide the crossfader towards B. Press ? for every key.");
                done = ui.button("Got it").clicked();
            });
        });
    done
}

/// The keyboard cheat sheet. Returns true when closed.
fn cheat_sheet(ui: &mut Ui, over: Rect) -> bool {
    let mut close = false;
    egui::Area::new(egui::Id::unique("dj_cheat_sheet"))
        .fixed_pos(over.min + Vec2::splat(4.0))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_size(over.size() - Vec2::splat(16.0));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("DJ keys").strong());
                    close = ui.small_button("✕").clicked();
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("dj_keys").striped(true).show(ui, |ui| {
                        for b in keymap() {
                            ui.label(RichText::new(key_label(&b)).monospace());
                            ui.label(describe(b.action));
                            ui.end_row();
                        }
                    });
                });
            });
        });
    close
}

/// Short-lived feedback ("press again to stop") over the waveforms.
fn message(ui: &Ui, st: &mut ConsoleState, over: Rect, now: Instant) {
    let Some((text, at)) = st.message.clone() else {
        return;
    };
    if now.duration_since(at) > Duration::from_secs(2) {
        st.message = None;
        return;
    }
    let painter = ui.painter_at(over);
    let pos = over.center_bottom() - Vec2::new(0.0, 4.0);
    painter.text(
        pos,
        egui::Align2::CENTER_BOTTOM,
        text,
        egui::FontId::proportional(11.0),
        Color32::from_rgb(255, 200, 60),
    );
}
