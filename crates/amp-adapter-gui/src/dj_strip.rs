//! Deck strips and the mixer column of the DJ console (TASK-212). Every button triggers a
//! [`DjAction`] through `act` or `hold`, which also record it, so a test can check that
//! every control is on screen.

#![forbid(unsafe_code)]

use crate::dj_controls::{
    describe, key_label, keymap, press, release, ConsoleState, DjAction, LOOP_SIZES,
};
use crate::dj_session::DjSession;
use crate::dj_waveform::{overview, Marks, CUE_COLORS};
use crate::responsive::{row_rtl, ROW};
use amp_core_engine::dj::engine::CueRouting;
use eframe::egui::{self, Color32, Rect, Response, RichText, Ui, Vec2};
use std::time::Instant;

/// Deck colours (A, B).
pub const DECK_COLORS: [Color32; 2] = [
    Color32::from_rgb(0, 200, 255),
    Color32::from_rgb(255, 150, 40),
];

/// Requests that need the whole app.
pub(crate) enum UiOut {
    LoadNext(usize),
    ToggleAuto,
    Leave,
}

/// What the console widgets work on.
pub(crate) struct Panel<'a> {
    pub dj: &'a mut DjSession,
    pub st: &'a mut ConsoleState,
    pub out: &'a mut Vec<UiOut>,
    pub now: Instant,
}

impl Panel<'_> {
    fn trigger(&mut self, a: DjAction, shift: bool) {
        if press(self.dj, self.st, a, shift, self.now) {
            self.out.push(UiOut::ToggleAuto);
        }
    }
}

fn hover(a: DjAction) -> String {
    let keys: Vec<String> = keymap()
        .iter()
        .filter(|b| b.action == a)
        .map(key_label)
        .collect();
    format!("{} ({})", describe(a), keys.join(", "))
}

/// A button for `a`: click (Shift for a bigger step) runs it.
pub(crate) fn act(
    p: &mut Panel<'_>,
    ui: &mut Ui,
    a: DjAction,
    label: RichText,
    on: bool,
) -> Response {
    p.st.rendered.push(a);
    let r = ui
        .add(egui::Button::selectable(on, label))
        .on_hover_text(hover(a));
    if r.clicked() {
        let shift = ui.input(|i| i.modifiers.shift);
        p.trigger(a, shift);
    }
    r
}

/// A button for a held control (nudge, cue preview): pressed while the mouse button is down.
pub(crate) fn hold(p: &mut Panel<'_>, ui: &mut Ui, a: DjAction, label: RichText) {
    p.st.rendered.push(a);
    let r = ui
        .add(
            egui::Button::new(label)
                .small()
                .sense(egui::Sense::click_and_drag()),
        )
        .on_hover_text(hover(a));
    let down = r.is_pointer_button_down_on();
    let was = p.st.held.contains(&a);
    if down && !was {
        p.st.held.push(a);
        p.trigger(a, false);
    } else if !down && was {
        p.st.held.retain(|h| *h != a);
        release(p.dj, p.st, a);
    }
}

/// One deck strip: title, readouts, overview, transport, loops, hot cues, tempo.
pub(crate) fn deck_strip(p: &mut Panel<'_>, ui: &mut Ui, deck: usize) {
    ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
    ui.spacing_mut().button_padding = Vec2::new(3.0, 1.0);
    let color = DECK_COLORS[deck];
    let title =
        p.dj.deck(deck)
            .map_or("Empty: load a track".to_string(), |s| {
                s.track.metadata.title.clone()
            });
    row_rtl(ui, |ui| {
        let load = ui
            .small_button("⏏")
            .on_hover_text("Load the next track assigned to this deck (or the selected one)");
        if load.clicked() {
            p.out.push(UiOut::LoadNext(deck));
        }
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(RichText::new(["A", "B"][deck]).strong().color(color));
            let t = RichText::new(title).color(Color32::from_rgb(220, 230, 240));
            ui.add(egui::Label::new(t).truncate());
        });
    });
    let s = p.dj.status().deck(deck);
    let (pos, bpm) = (s.position_secs(), s.bpm());
    let total =
        p.dj.deck(deck)
            .and_then(|d| d.analysis().map(|a| a.duration_secs))
            .unwrap_or(s.loaded_secs());
    let pitch = p.dj.params().deck(deck).pitch_percent();
    row_rtl(ui, |ui| {
        let tempo = if bpm > 0.0 {
            format!("{bpm:.1}")
        } else {
            "—".into()
        };
        ui.label(
            RichText::new(format!("{tempo} BPM"))
                .monospace()
                .size(10.0)
                .color(color),
        );
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
    let rows = 4.0 * (ROW + 3.0);
    let wave_h = (ui.available_height() - rows).max(14.0);
    let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), wave_h));
    let cues: [Option<f64>; 8] = std::array::from_fn(|i| s.hot_cue(i));
    let marks = Marks {
        cues: &cues,
        loop_range: s.loop_range(),
        loop_active: s.loop_active(),
    };
    let looping = s.loop_active();
    let analysis = p.dj.deck(deck).and_then(|d| d.analysis().cloned());
    if let Some(t) = overview(ui, rect, analysis.as_ref(), pos, total, color, marks) {
        p.dj.control().seek(deck, t);
    }

    let params = p.dj.control().params_handle();
    let d = params.deck(deck);
    let (playing, sync, pfl) = (d.playing(), d.sync(), d.pfl());
    ui.horizontal(|ui| {
        act(
            p,
            ui,
            DjAction::PlayPause(deck),
            RichText::new(if playing { "⏸" } else { "▶" }),
            playing,
        );
        hold(p, ui, DjAction::Cue(deck), RichText::new("CUE").size(10.0));
        act(
            p,
            ui,
            DjAction::Sync(deck),
            RichText::new("SYNC").size(10.0),
            sync,
        );
        act(p, ui, DjAction::Pfl(deck), RichText::new("🎧"), pfl);
    });
    ui.horizontal(|ui| {
        act(
            p,
            ui,
            DjAction::LoopAb(deck),
            RichText::new("A-B").size(10.0),
            false,
        );
        act(
            p,
            ui,
            DjAction::LoopHalve(deck),
            RichText::new("½").size(10.0),
            false,
        );
        let size = LOOP_SIZES[p.st.loop_size[deck]];
        let label = if size < 1.0 {
            format!("1/{}", (1.0 / size) as u32)
        } else {
            format!("{size}")
        };
        act(
            p,
            ui,
            DjAction::BeatLoop(deck),
            RichText::new(label).size(10.0),
            looping,
        );
        act(
            p,
            ui,
            DjAction::LoopDouble(deck),
            RichText::new("×2").size(10.0),
            false,
        );
        act(
            p,
            ui,
            DjAction::ExitLoop(deck),
            RichText::new("🗙").size(10.0),
            false,
        );
    });
    ui.horizontal(|ui| {
        for (slot, cue) in cues.iter().enumerate() {
            let c = if cue.is_some() {
                CUE_COLORS[slot]
            } else {
                Color32::from_gray(110)
            };
            let r = act(
                p,
                ui,
                DjAction::HotCue(deck, slot),
                RichText::new(format!("{}", slot + 1)).size(9.0).color(c),
                false,
            );
            p.st.rendered.push(DjAction::ClearHotCue(deck, slot));
            if r.secondary_clicked() {
                p.trigger(DjAction::ClearHotCue(deck, slot), false);
            }
        }
    });
    ui.horizontal(|ui| {
        hold(
            p,
            ui,
            DjAction::NudgeBack(deck),
            RichText::new("«").size(10.0),
        );
        p.st.rendered
            .extend([DjAction::PitchDown(deck), DjAction::PitchUp(deck)]);
        let mut pitch = d.pitch_percent();
        ui.spacing_mut().slider_width = (ui.available_width() - 24.0).max(30.0);
        let slider = egui::Slider::new(&mut pitch, -8.0..=8.0).show_value(false);
        if ui
            .add(slider)
            .on_hover_text(hover(DjAction::PitchUp(deck)))
            .changed()
        {
            d.set_pitch_percent(pitch);
        }
        hold(
            p,
            ui,
            DjAction::NudgeForward(deck),
            RichText::new("»").size(10.0),
        );
    });
}

/// The mixer column: exit and auto, channel faders, crossfader, headphones, help.
pub(crate) fn mixer_column(p: &mut Panel<'_>, ui: &mut Ui, width: f32) {
    ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            let exit = ui
                .small_button("Exit")
                .on_hover_text("Back to normal playback (Ctrl+J)");
            if exit.clicked() {
                p.out.push(UiOut::Leave);
            }
            let auto = p.dj.automix_on();
            act(
                p,
                ui,
                DjAction::AutoMix,
                RichText::new("AUTO").size(10.0),
                auto,
            );
            act(
                p,
                ui,
                DjAction::CheatSheet,
                RichText::new("?").size(10.0),
                p.st.cheat_sheet,
            );
        });
        let params = p.dj.control().params_handle();
        for deck in 0..2 {
            let mut fader = params.deck(deck).fader();
            ui.spacing_mut().slider_width = width - 24.0;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(["A", "B"][deck])
                        .size(10.0)
                        .color(DECK_COLORS[deck]),
                );
                if ui
                    .add(egui::Slider::new(&mut fader, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    params.deck(deck).set_fader(fader);
                }
            });
        }
        let mut xf = params.crossfader();
        ui.spacing_mut().slider_width = width - 22.0;
        p.st.rendered
            .extend([DjAction::CrossfadeLeft, DjAction::CrossfadeRight]);
        ui.horizontal(|ui| {
            let r = ui.add(egui::Slider::new(&mut xf, -1.0..=1.0).show_value(false));
            if r.on_hover_text(hover(DjAction::CrossfadeRight)).changed() {
                params.set_crossfader(xf);
            }
            act(
                p,
                ui,
                DjAction::CrossfadeCentre,
                RichText::new("·").size(10.0),
                false,
            );
        });
        let mix = params.cue_mix();
        let (mut m, mut v) = (mix.mix, mix.volume);
        ui.spacing_mut().slider_width = width - 24.0;
        p.st.rendered
            .extend([DjAction::CueMixDown, DjAction::CueMixUp]);
        ui.horizontal(|ui| {
            ui.label(RichText::new("🎧").size(10.0))
                .on_hover_text("Headphones: cue ◀ ▶ master");
            if ui
                .add(egui::Slider::new(&mut m, 0.0..=1.0).show_value(false))
                .on_hover_text(hover(DjAction::CueMixUp))
                .changed()
            {
                params.set_cue_mix(m);
            }
        });
        p.st.rendered
            .extend([DjAction::CueVolumeDown, DjAction::CueVolumeUp]);
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔈").size(10.0))
                .on_hover_text("Headphone level");
            if ui
                .add(egui::Slider::new(&mut v, 0.0..=1.0).show_value(false))
                .on_hover_text(hover(DjAction::CueVolumeUp))
                .changed()
            {
                params.set_cue_volume(v);
            }
        });
        let routing = params.cue_routing();
        egui::ComboBox::from_id_salt("dj_cue_routing")
            .width(width - 8.0)
            .selected_text(routing_name(routing))
            .show_ui(ui, |ui| {
                for r in [CueRouting::Off, CueRouting::Split, CueRouting::SecondDevice] {
                    if ui.selectable_label(routing == r, routing_name(r)).clicked() {
                        params.set_cue_routing(r);
                    }
                }
            });
    });
}

fn routing_name(r: CueRouting) -> &'static str {
    match r {
        CueRouting::Off => "Cue: off",
        CueRouting::Split => "Cue: split L/R",
        CueRouting::ExtraChannels => "Cue: outputs 3/4",
        CueRouting::SecondDevice => "Cue: 2nd device",
    }
}

fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
