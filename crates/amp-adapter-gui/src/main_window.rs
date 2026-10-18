//! Main player faceplate window for Multiplay.
//! Classic Winamp 1.x VFD glowing display, real-time visualizer, transport controls, and timeline scrubber.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use crate::docking::WindowId;
use crate::visualizer_widget::VisualizerWidget;
use eframe::egui::{self, Color32, RichText, Ui};
use std::time::Duration;

pub fn render_main_window_content(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    ui.group(|ui| {
        // 1. Title Bar & VFD Track Title
        ui.horizontal(|ui| {
            let title_text =
                if !app.playlist.is_empty() && app.selected_track_idx < app.playlist.len() {
                    let track = &app.playlist[app.selected_track_idx];
                    format!(
                        "WINAMP 1.x :: {}. {} - {}",
                        app.selected_track_idx + 1,
                        track.metadata.artist,
                        track.metadata.title
                    )
                } else {
                    "WINAMP 1.x :: [NO TRACK LOADED - DROP FILES HERE]".to_string()
                };
            ui.heading(
                RichText::new(title_text)
                    .size(14.0)
                    .strong()
                    .color(Color32::from_rgb(0, 235, 136)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.remote.is_running {
                    let text = if app.remote.allow_lan {
                        "● REMOTE ON (LAN)"
                    } else {
                        "● REMOTE ON"
                    };
                    ui.label(
                        RichText::new(text)
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(240, 180, 0)),
                    )
                    .on_hover_text(
                        "Remote control is listening. Turn it off in Settings → Remote.",
                    );
                }
                ui.label(
                    RichText::new("[44.1kHz / 24-bit] [ST]")
                        .size(11.0)
                        .color(Color32::from_rgb(0, 200, 240)),
                );
            });
        });

        ui.add_space(4.0);

        // 2. Hardware Output Device Selector
        let current_dev = app.audio_driver.current_device_name.lock().unwrap().clone();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🔊 AUDIO DEVICE:")
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(140, 170, 190)),
            );
            for dev in &app.audio_driver.available_devices {
                let is_selected = dev.name == current_dev;
                let btn_text = if is_selected {
                    format!("● {}", dev.name)
                } else {
                    format!("○ {}", dev.name)
                };
                let text_color = if is_selected {
                    Color32::from_rgb(0, 255, 180)
                } else {
                    Color32::from_rgb(140, 140, 150)
                };
                if ui
                    .selectable_label(
                        is_selected,
                        RichText::new(btn_text).size(10.0).color(text_color),
                    )
                    .clicked()
                    && !is_selected
                {
                    let _ = app.audio_driver.switch_device(&dev.name);
                }
            }
        });

        ui.add_space(4.0);

        // 3. Glowing VFD Digital Clock & Visualizer
        ui.horizontal(|ui| {
            let elapsed_dur = *app.audio_driver.elapsed.lock().unwrap();
            let el_s = elapsed_dur.as_secs();
            let clock_text = format!("{:02}:{:02}", el_s / 60, el_s % 60);

            ui.vertical(|ui| {
                ui.label(
                    RichText::new("ELAPSED TIME")
                        .size(9.0)
                        .color(Color32::from_rgb(100, 140, 160)),
                );
                ui.label(
                    RichText::new(clock_text)
                        .size(26.0)
                        .monospace()
                        .strong()
                        .color(Color32::from_rgb(0, 255, 140)),
                );
            });

            ui.add_space(10.0);

            let spec = *app.audio_driver.visualizer_spectrum.lock().unwrap();
            let wave = *app.audio_driver.visualizer_waveform.lock().unwrap();
            let vis_resp =
                VisualizerWidget::new(app.visualizer_mode, &spec, &wave, &app.theme, 42.0).ui(ui);
            if vis_resp.clicked() {
                app.visualizer_mode = match app.visualizer_mode {
                    crate::visualizer_widget::VisualizerMode::Spectrum => {
                        crate::visualizer_widget::VisualizerMode::Oscilloscope
                    }
                    crate::visualizer_widget::VisualizerMode::Oscilloscope => {
                        crate::visualizer_widget::VisualizerMode::Spectrum
                    }
                };
            }
        });

        ui.add_space(4.0);

        // 4. Interactive Timeline Scrubbing Bar
        let elapsed = *app.audio_driver.elapsed.lock().unwrap();
        let total = *app.audio_driver.total_duration.lock().unwrap();
        let total_secs = total
            .unwrap_or(Duration::from_secs(1))
            .as_secs_f32()
            .max(1.0);
        let mut progress = (elapsed.as_secs_f32() / total_secs).clamp(0.0, 1.0);

        ui.horizontal(|ui| {
            let el_str = format!(
                "{:02}:{:02}",
                elapsed.as_secs() / 60,
                elapsed.as_secs() % 60
            );
            let tot_str = if let Some(t) = total {
                format!("{:02}:{:02}", t.as_secs() / 60, t.as_secs() % 60)
            } else {
                "--:--".to_string()
            };

            ui.label(
                RichText::new(el_str)
                    .size(10.0)
                    .monospace()
                    .color(Color32::from_rgb(0, 230, 140)),
            );
            let scrub_slider = egui::Slider::new(&mut progress, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true);
            if ui
                .add_sized([ui.available_width() - 50.0, 12.0], scrub_slider)
                .changed()
            {
                let target = Duration::from_secs_f32(progress * total_secs);
                app.audio_driver.seek(target);
            }
            ui.label(
                RichText::new(tot_str)
                    .size(10.0)
                    .monospace()
                    .color(Color32::from_rgb(120, 180, 240)),
            );
        });

        ui.add_space(6.0);

        // 5. Transport Controls Row & Window Toggles
        ui.horizontal(|ui| {
            if ui.button(RichText::new("⏮ PREV").size(12.0)).clicked() && app.selected_track_idx > 0
            {
                app.selected_track_idx -= 1;
                app.audio_driver
                    .play(app.playlist[app.selected_track_idx].clone());
                let upcoming = app.playlist.get(app.selected_track_idx + 1).cloned();
                app.audio_driver.set_next_track(upcoming);
            }

            let is_playing = app
                .audio_driver
                .is_playing
                .load(std::sync::atomic::Ordering::Relaxed);
            let play_label = if is_playing { "⏸ PAUSE" } else { "▶ PLAY" };
            let play_color = if is_playing {
                Color32::from_rgb(255, 180, 50)
            } else {
                Color32::from_rgb(0, 230, 140)
            };

            if ui
                .button(
                    RichText::new(play_label)
                        .size(13.0)
                        .strong()
                        .color(play_color),
                )
                .clicked()
            {
                if !is_playing {
                    if !app.playlist.is_empty() && app.selected_track_idx < app.playlist.len() {
                        app.audio_driver
                            .play(app.playlist[app.selected_track_idx].clone());
                        let upcoming = app.playlist.get(app.selected_track_idx + 1).cloned();
                        app.audio_driver.set_next_track(upcoming);
                    }
                } else {
                    app.audio_driver.toggle_play_pause();
                }
            }

            if ui.button(RichText::new("⏹ STOP").size(12.0)).clicked() {
                app.audio_driver.stop();
            }

            if ui.button(RichText::new("⏭ NEXT").size(12.0)).clicked()
                && app.selected_track_idx + 1 < app.playlist.len()
            {
                app.selected_track_idx += 1;
                app.audio_driver
                    .play(app.playlist[app.selected_track_idx].clone());
                let upcoming = app.playlist.get(app.selected_track_idx + 1).cloned();
                app.audio_driver.set_next_track(upcoming);
            }

            ui.add_space(10.0);

            // Volume Slider
            ui.label(
                RichText::new("VOL:")
                    .size(11.0)
                    .color(Color32::from_rgb(180, 200, 220)),
            );
            let vol_slider = egui::Slider::new(&mut app.volume, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true);
            if ui.add_sized([75.0, 16.0], vol_slider).changed() {
                app.audio_driver.set_volume(app.volume);
            }

            // Window Toggles
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let pref_on = app.dock_manager.is_visible(WindowId::Settings);
                if ui
                    .button(
                        RichText::new(if pref_on { "PREF [ON]" } else { "PREF [OFF]" }).size(10.0),
                    )
                    .clicked()
                {
                    app.dock_manager.toggle_visible(WindowId::Settings);
                }
                let dj_on = app.dock_manager.is_visible(WindowId::DjConsole);
                if ui
                    .button(RichText::new(if dj_on { "DJ [ON]" } else { "DJ [OFF]" }).size(10.0))
                    .clicked()
                {
                    app.dock_manager.toggle_visible(WindowId::DjConsole);
                }
                let pl_on = app.dock_manager.is_visible(WindowId::Playlist);
                if ui
                    .button(RichText::new(if pl_on { "PL [ON]" } else { "PL [OFF]" }).size(10.0))
                    .clicked()
                {
                    app.dock_manager.toggle_visible(WindowId::Playlist);
                }
                let eq_on = app.dock_manager.is_visible(WindowId::Equalizer);
                if ui
                    .button(RichText::new(if eq_on { "EQ [ON]" } else { "EQ [OFF]" }).size(10.0))
                    .clicked()
                {
                    app.dock_manager.toggle_visible(WindowId::Equalizer);
                }
            });
        });
    });
}
