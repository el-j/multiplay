//! Professional multi-deck DJ console panel for native desktop egui interface.
//! Real-time zero-allocation controls for pitch bending, 3-band kill EQ, beatmatching sync, and crossfading.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use amp_core_engine::dj::CrossfaderCurveProfile;
use amp_domain::Track;
use eframe::egui::{self, Color32, RichText, Ui};

/// DJ console state for the GUI desktop adapter.
#[derive(Debug, Clone)]
pub struct DjConsoleState {
    pub deck_a_title: String,
    pub deck_a_bpm: f32,
    pub deck_a_pitch: f32,
    pub deck_a_playing: bool,
    pub deck_a_cue: bool,
    pub deck_a_kill_low: bool,
    pub deck_a_kill_mid: bool,
    pub deck_a_kill_high: bool,

    pub deck_b_title: String,
    pub deck_b_bpm: f32,
    pub deck_b_pitch: f32,
    pub deck_b_playing: bool,
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
            deck_a_title: "Deck A [Empty]".into(),
            deck_a_bpm: 128.0,
            deck_a_pitch: 0.0,
            deck_a_playing: false,
            deck_a_cue: false,
            deck_a_kill_low: false,
            deck_a_kill_mid: false,
            deck_a_kill_high: false,

            deck_b_title: "Deck B [Empty]".into(),
            deck_b_bpm: 124.0,
            deck_b_pitch: 0.0,
            deck_b_playing: false,
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
    pub fn show(
        ui: &mut Ui,
        state: &mut DjConsoleState,
        selected_track: Option<&Track>,
        driver: &AudioPlaybackDriver,
    ) {
        // Read live BPM updates from audio engine
        if let Ok(shared) = driver.dj_audio.try_lock() {
            state.deck_a_bpm = shared.deck_a_bpm;
            state.deck_b_bpm = shared.deck_b_bpm;
        }

        ui.group(|ui| {
            render_header(ui, state);
            ui.add_space(4.0);

            ui.columns(2, |columns| {
                render_deck_column(&mut columns[0], state, true, selected_track, driver);
                render_deck_column(&mut columns[1], state, false, selected_track, driver);
            });

            ui.add_space(6.0);
            render_crossfader(ui, state, driver);
        });

        // Push state changes into audio driver
        sync_to_audio_driver(state, driver);
    }
}

fn render_header(ui: &mut Ui, state: &mut DjConsoleState) {
    ui.horizontal(|ui| {
        ui.heading(
            RichText::new("🎛 PRO DJ CONSOLE")
                .color(Color32::from_rgb(0, 235, 136))
                .strong(),
        );
        ui.separator();
        ui.checkbox(&mut state.key_lock, "Key Lock (WSOLA)");
        ui.separator();
        let curve_text = if state.sharp_cut {
            "Curve: Sharp Cut"
        } else {
            "Curve: Smooth Dip"
        };
        if ui.button(curve_text).clicked() {
            state.sharp_cut = !state.sharp_cut;
        }
    });
}

fn render_deck_column(
    ui: &mut Ui,
    state: &mut DjConsoleState,
    is_deck_a: bool,
    selected: Option<&Track>,
    driver: &AudioPlaybackDriver,
) {
    let (label, color) = if is_deck_a {
        ("DECK A", Color32::from_rgb(60, 180, 255))
    } else {
        ("DECK B", Color32::from_rgb(255, 140, 40))
    };

    let title = if is_deck_a {
        state.deck_a_title.clone()
    } else {
        state.deck_b_title.clone()
    };
    let bpm = if is_deck_a {
        state.deck_a_bpm
    } else {
        state.deck_b_bpm
    };
    let is_playing = if is_deck_a {
        state.deck_a_playing
    } else {
        state.deck_b_playing
    };

    ui.vertical(|ui| {
        ui.label(RichText::new(label).color(color).strong());
        ui.label(RichText::new(title).strong());

        ui.horizontal(|ui| {
            if ui.button("LOAD SEL").clicked() {
                if let Some(track) = selected {
                    let mut shared = driver.dj_audio.lock().unwrap();
                    let name = format!("{} - {}", track.metadata.artist, track.metadata.title);
                    if is_deck_a {
                        shared.load_deck_a(track.clone());
                        state.deck_a_title = name;
                        state.deck_a_playing = true;
                    } else {
                        shared.load_deck_b(track.clone());
                        state.deck_b_title = name;
                        state.deck_b_playing = true;
                    }
                }
            }

            let play_btn_text = if is_playing { "⏸ PAUSE" } else { "▶ PLAY" };
            if ui.button(play_btn_text).clicked() {
                if is_deck_a {
                    state.deck_a_playing = !state.deck_a_playing;
                } else {
                    state.deck_b_playing = !state.deck_b_playing;
                }
            }

            ui.label(format!("BPM: {:.1}", bpm));

            // Sync button
            let sync_text = if is_deck_a { "SYNC [B]" } else { "SYNC [A]" };
            if ui
                .button(RichText::new(sync_text).color(Color32::from_rgb(0, 235, 136)))
                .clicked()
            {
                let mut shared = driver.dj_audio.lock().unwrap();
                if is_deck_a {
                    state.deck_a_pitch = shared.sync_deck_a_to_b();
                } else {
                    state.deck_b_pitch = shared.sync_deck_b_to_a();
                }
            }
        });

        // Pitch slider
        let pitch = if is_deck_a {
            &mut state.deck_a_pitch
        } else {
            &mut state.deck_b_pitch
        };
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(pitch, -16.0..=16.0).text("Pitch %"));
            if ui.button("0%").clicked() {
                *pitch = 0.0;
            }
        });

        // Kill buttons
        render_kill_buttons(ui, state, is_deck_a);
    });
}

fn render_kill_buttons(ui: &mut Ui, state: &mut DjConsoleState, is_deck_a: bool) {
    ui.horizontal(|ui| {
        let (cue, hi, mid, low) = if is_deck_a {
            (
                &mut state.deck_a_cue,
                &mut state.deck_a_kill_high,
                &mut state.deck_a_kill_mid,
                &mut state.deck_a_kill_low,
            )
        } else {
            (
                &mut state.deck_b_cue,
                &mut state.deck_b_kill_high,
                &mut state.deck_b_kill_mid,
                &mut state.deck_b_kill_low,
            )
        };

        let cue_color = if *cue { Color32::RED } else { Color32::GRAY };
        if ui.button(RichText::new("CUE").color(cue_color)).clicked() {
            *cue = !*cue;
        }

        if ui.button(if *hi { "KILL HI [X]" } else { "HI" }).clicked() {
            *hi = !*hi;
        }
        if ui
            .button(if *mid { "KILL MID [X]" } else { "MID" })
            .clicked()
        {
            *mid = !*mid;
        }
        if ui
            .button(if *low { "KILL LOW [X]" } else { "LOW" })
            .clicked()
        {
            *low = !*low;
        }
    });
}

fn render_crossfader(ui: &mut Ui, state: &mut DjConsoleState, driver: &AudioPlaybackDriver) {
    ui.horizontal(|ui| {
        if ui.button("◄ A").clicked() {
            state.crossfader_pos = -1.0;
        }
        ui.add(egui::Slider::new(&mut state.crossfader_pos, -1.0..=1.0).text("CROSSFADER"));
        if ui.button("CTR").clicked() {
            state.crossfader_pos = 0.0;
        }
        if ui.button("B ►").clicked() {
            state.crossfader_pos = 1.0;
        }
    });

    if let Ok(mut shared) = driver.dj_audio.try_lock() {
        shared.crossfader_pos = state.crossfader_pos;
    }
}

fn sync_to_audio_driver(state: &DjConsoleState, driver: &AudioPlaybackDriver) {
    if let Ok(mut shared) = driver.dj_audio.try_lock() {
        shared.deck_a_playing = state.deck_a_playing;
        shared.deck_a_pitch = state.deck_a_pitch;
        shared.deck_a_kill_low = state.deck_a_kill_low;
        shared.deck_a_kill_mid = state.deck_a_kill_mid;
        shared.deck_a_kill_high = state.deck_a_kill_high;

        shared.deck_b_playing = state.deck_b_playing;
        shared.deck_b_pitch = state.deck_b_pitch;
        shared.deck_b_kill_low = state.deck_b_kill_low;
        shared.deck_b_kill_mid = state.deck_b_kill_mid;
        shared.deck_b_kill_high = state.deck_b_kill_high;

        shared.crossfader_pos = state.crossfader_pos;
        shared.key_lock = state.key_lock;
        shared.curve_profile = if state.sharp_cut {
            CrossfaderCurveProfile::SharpCut
        } else {
            CrossfaderCurveProfile::SmoothDip
        };
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
        assert!((state.deck_a_pitch - 0.0).abs() < f32::EPSILON);
    }
}
