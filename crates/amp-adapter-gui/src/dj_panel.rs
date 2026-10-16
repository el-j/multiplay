//! Professional multi-deck DJ console panel for native desktop egui interface.

#![forbid(unsafe_code)]

use eframe::egui::{self, Color32, RichText, Ui};

/// DJ console state for the GUI desktop adapter.
#[derive(Debug, Clone)]
pub struct DjConsoleState {
    pub deck_a_title: String,
    pub deck_a_bpm: f32,
    pub deck_a_pitch: f32,
    pub deck_a_cue: bool,
    pub deck_a_kill_low: bool,
    pub deck_a_kill_mid: bool,
    pub deck_a_kill_high: bool,

    pub deck_b_title: String,
    pub deck_b_bpm: f32,
    pub deck_b_pitch: f32,
    pub deck_b_cue: bool,
    pub deck_b_kill_low: bool,
    pub deck_b_kill_mid: bool,
    pub deck_b_kill_high: bool,

    pub crossfader_pos: f32,
    pub sharp_cut: bool,
    pub key_lock: bool,
}

impl Default for DjConsoleState {
    fn default() -> Self {
        Self {
            deck_a_title: "Aphex Twin - Polynomial-C".into(),
            deck_a_bpm: 128.0,
            deck_a_pitch: 0.0,
            deck_a_cue: false,
            deck_a_kill_low: false,
            deck_a_kill_mid: false,
            deck_a_kill_high: false,

            deck_b_title: "Kraftwerk - Computer World".into(),
            deck_b_bpm: 124.0,
            deck_b_pitch: 0.0,
            deck_b_cue: false,
            deck_b_kill_low: false,
            deck_b_kill_mid: false,
            deck_b_kill_high: false,

            crossfader_pos: 0.0,
            sharp_cut: false,
            key_lock: true,
        }
    }
}

pub struct DjConsolePanel;

impl DjConsolePanel {
    pub fn show(ui: &mut Ui, state: &mut DjConsoleState) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new("🎛 PRO DJ CONSOLE")
                        .color(Color32::from_rgb(0, 235, 136))
                        .strong(),
                );
                ui.separator();
                ui.checkbox(&mut state.key_lock, "Key Lock (Master Tempo)");
                ui.separator();
                let curve_text = if state.sharp_cut { "Curve: Sharp Cut" } else { "Curve: Smooth Dip" };
                if ui.button(curve_text).clicked() {
                    state.sharp_cut = !state.sharp_cut;
                }
            });

            ui.add_space(6.0);

            ui.columns(2, |columns| {
                // Deck A Column
                columns[0].vertical(|ui| {
                    ui.label(
                        RichText::new("DECK A")
                            .color(Color32::from_rgb(60, 180, 255))
                            .strong(),
                    );
                    ui.label(RichText::new(&state.deck_a_title).strong());
                    ui.label(format!("BPM: {:.1}", state.deck_a_bpm));
                    ui.add(egui::Slider::new(&mut state.deck_a_pitch, -8.0..=8.0).text("Pitch %"));

                    ui.horizontal(|ui| {
                        let cue_color = if state.deck_a_cue { Color32::RED } else { Color32::GRAY };
                        if ui.button(RichText::new("CUE PFL").color(cue_color)).clicked() {
                            state.deck_a_cue = !state.deck_a_cue;
                        }
                        if ui.button(if state.deck_a_kill_high { "KILL HI [X]" } else { "HI" }).clicked() {
                            state.deck_a_kill_high = !state.deck_a_kill_high;
                        }
                        if ui.button(if state.deck_a_kill_mid { "KILL MID [X]" } else { "MID" }).clicked() {
                            state.deck_a_kill_mid = !state.deck_a_kill_mid;
                        }
                        if ui.button(if state.deck_a_kill_low { "KILL LOW [X]" } else { "LOW" }).clicked() {
                            state.deck_a_kill_low = !state.deck_a_kill_low;
                        }
                    });
                });

                // Deck B Column
                columns[1].vertical(|ui| {
                    ui.label(
                        RichText::new("DECK B")
                            .color(Color32::from_rgb(255, 140, 40))
                            .strong(),
                    );
                    ui.label(RichText::new(&state.deck_b_title).strong());
                    ui.label(format!("BPM: {:.1}", state.deck_b_bpm));
                    ui.add(egui::Slider::new(&mut state.deck_b_pitch, -8.0..=8.0).text("Pitch %"));

                    ui.horizontal(|ui| {
                        let cue_color = if state.deck_b_cue { Color32::RED } else { Color32::GRAY };
                        if ui.button(RichText::new("CUE PFL").color(cue_color)).clicked() {
                            state.deck_b_cue = !state.deck_b_cue;
                        }
                        if ui.button(if state.deck_b_kill_high { "KILL HI [X]" } else { "HI" }).clicked() {
                            state.deck_b_kill_high = !state.deck_b_kill_high;
                        }
                        if ui.button(if state.deck_b_kill_mid { "KILL MID [X]" } else { "MID" }).clicked() {
                            state.deck_b_kill_mid = !state.deck_b_kill_mid;
                        }
                        if ui.button(if state.deck_b_kill_low { "KILL LOW [X]" } else { "LOW" }).clicked() {
                            state.deck_b_kill_low = !state.deck_b_kill_low;
                        }
                    });
                });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("◄ DECK A");
                ui.add(egui::Slider::new(&mut state.crossfader_pos, -1.0..=1.0).text("CROSSFADER"));
                ui.label("DECK B ►");
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dj_panel_state() {
        let mut state = DjConsoleState::default();
        assert_eq!(state.crossfader_pos, 0.0);
        assert!(!state.deck_a_cue);

        state.deck_a_cue = true;
        assert!(state.deck_a_cue);
    }
}
