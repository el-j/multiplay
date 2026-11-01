//! Main player faceplate: track display, output device, clock and visualizer, scrubber,
//! transport and window toggles. Responsive from 300 pt wide: text truncates, the transport
//! turns icon-only when narrow, and the visualizer takes all spare width and height.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use crate::docking::WindowId;
use crate::responsive::row_rtl;
use crate::transport::TransportAction;
use crate::visualizer_widget::{VisualizerMode, VisualizerWidget};
use eframe::egui::{self, Color32, Layout, RichText, Ui};
use std::time::Duration;

/// Below this content width the transport shows icons only.
const NARROW: f32 = 440.0;
/// Height reserved under the visualizer for scrubber, transport and toggle rows.
const BOTTOM_ROWS: f32 = 78.0;
const ACCENT: Color32 = Color32::from_rgb(0, 235, 136);

pub fn render_main_window_content(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    if ctrl_j(ui) {
        app.toggle_dj_mode(Some(ui.ctx()));
    }
    if app.dj_active() {
        crate::dj_console::render_dj_console(app, ui);
        return;
    }
    let narrow = ui.available_width() < NARROW;
    ui.spacing_mut().item_spacing.y = 4.0;
    track_row(app, ui, narrow);
    device_row(app, ui);
    clock_and_visualizer_row(app, ui);
    scrubber_row(app, ui);
    transport_row(app, ui, narrow);
    toggles_row(app, ui);
}

/// `44.1 kHz · 16-bit · ST · BIT-PERFECT`: the source rate from the decoder, depth (lossless) or
/// bitrate (lossy) and channels from the tags, and what the output does with it. Empty when
/// nothing is decoding, so the faceplate never shows a made-up format.
#[must_use]
pub fn stream_label(
    info: Option<&amp_core_engine::player::StreamInfo>,
    meta: &amp_domain::TrackMetadata,
) -> String {
    let Some(info) = info else {
        return String::new();
    };
    let khz = |hz: u32| {
        let k = f64::from(hz) / 1000.0;
        if k.fract().abs() < 1e-9 {
            format!("{k:.0} kHz")
        } else {
            format!("{k:.1} kHz")
        }
    };
    let mut parts = vec![khz(info.source_rate)];
    if let Some(bits) = meta.bit_depth {
        parts.push(format!("{bits}-bit"));
    } else if let Some(kbps) = meta.bitrate_kbps {
        parts.push(format!("{kbps} kbps"));
    }
    match meta.channels {
        Some(1) => parts.push("MONO".into()),
        Some(2) => parts.push("ST".into()),
        Some(6) => parts.push("5.1".into()),
        Some(8) => parts.push("7.1".into()),
        Some(n) => parts.push(format!("{n} CH")),
        None => {}
    }
    parts.push(if info.bit_perfect {
        "BIT-PERFECT".into()
    } else if info.source_rate != info.output_rate {
        format!("DSP · {} OUT", khz(info.output_rate))
    } else {
        "DSP".into()
    });
    parts.join(" · ")
}

fn track_row(app: &MultiplayGuiApp, ui: &mut Ui, narrow: bool) {
    let title = match app.playlist.get(app.selected_track_idx) {
        Some(t) => format!(
            "{}. {} - {}",
            app.selected_track_idx + 1,
            t.metadata.artist,
            t.metadata.title
        ),
        None => "No track loaded - drop files here".to_string(),
    };
    row_rtl(ui, |ui| {
        if app.remote.is_running {
            let text = if app.remote.allow_lan {
                "⚫ REMOTE (LAN)"
            } else {
                "⚫ REMOTE"
            };
            ui.label(
                RichText::new(text)
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(240, 180, 0)),
            )
            .on_hover_text("Remote control is listening. Phones and requests: the 📱 window.");
        }
        if let amp_domain::PlaybackState::Buffering { progress_percent } =
            app.audio_driver.playback_state()
        {
            ui.label(
                RichText::new(format!("⏳ BUFFERING {progress_percent}%"))
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(240, 180, 0)),
            )
            .on_hover_text("Waiting for the stream to download");
        }
        if !narrow {
            let info = app.audio_driver.stream_info();
            let label = app
                .audio_driver
                .current_track
                .lock()
                .ok()
                .and_then(|t| t.as_ref().map(|t| stream_label(info.as_ref(), &t.metadata)))
                .unwrap_or_default();
            if !label.is_empty() {
                ui.label(
                    RichText::new(label)
                        .size(10.0)
                        .color(Color32::from_rgb(0, 200, 240)),
                )
                .on_hover_text(
                    "Source format. BIT-PERFECT: samples reach the output unchanged. DSP: \
                     resampled, or volume, EQ or loudness are changing them.",
                );
            }
        }
        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add(
                egui::Label::new(RichText::new(title).size(13.0).strong().color(ACCENT)).truncate(),
            )
            .on_hover_text("Current track");
        });
    });
}

fn device_row(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let current = app.audio_driver.current_device_name.lock().unwrap().clone();
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("🔊")
                .size(11.0)
                .color(Color32::from_rgb(140, 170, 190)),
        )
        .on_hover_text("Audio output device");
        egui::ComboBox::from_id_salt("output_device")
            .width(ui.available_width() - 8.0)
            .truncate()
            .selected_text(RichText::new(&current).size(10.0))
            .show_ui(ui, |ui| {
                for dev in &app.audio_driver.available_devices {
                    if ui
                        .selectable_label(dev.name == current, &dev.name)
                        .clicked()
                    {
                        chosen = Some(dev.name.clone());
                    }
                }
            });
    });
    if let Some(name) = chosen.filter(|n| *n != current) {
        let _ = app.audio_driver.switch_device(&name);
    }
}

fn clock_and_visualizer_row(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let height = (ui.available_height() - BOTTOM_ROWS).max(36.0);
    let el_s = app.audio_driver.elapsed.lock().unwrap().as_secs();
    ui.horizontal(|ui| {
        ui.set_height(height);
        ui.vertical(|ui| {
            ui.label(
                RichText::new("ELAPSED")
                    .size(9.0)
                    .color(Color32::from_rgb(100, 140, 160)),
            );
            ui.label(
                RichText::new(format!("{:02}:{:02}", el_s / 60, el_s % 60))
                    .size(24.0)
                    .monospace()
                    .strong()
                    .color(Color32::from_rgb(0, 255, 140)),
            );
        });
        let spec = *app.audio_driver.visualizer_spectrum.lock().unwrap();
        let wave = *app.audio_driver.visualizer_waveform.lock().unwrap();
        let resp =
            VisualizerWidget::new(app.visualizer_mode, &spec, &wave, &app.theme, height).ui(ui);
        if resp
            .on_hover_text("Click to switch spectrum / oscilloscope")
            .clicked()
        {
            app.visualizer_mode = match app.visualizer_mode {
                VisualizerMode::Spectrum => VisualizerMode::Oscilloscope,
                VisualizerMode::Oscilloscope => VisualizerMode::Spectrum,
            };
        }
    });
}

fn mmss(d: Duration) -> String {
    format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60)
}

fn scrubber_row(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let elapsed = *app.audio_driver.elapsed.lock().unwrap();
    let total = *app.audio_driver.total_duration.lock().unwrap();
    let total_secs = total.map_or(1.0, |t| t.as_secs_f32()).max(1.0);
    let playing_at = (elapsed.as_secs_f32() / total_secs).clamp(0.0, 1.0);
    let mut progress = app.scrub.shown(playing_at);
    let small = |s: String, c| RichText::new(s).size(10.0).monospace().color(c);
    row_rtl(ui, |ui| {
        let tot = total.map_or("--:--".to_string(), mmss);
        ui.label(small(tot, Color32::from_rgb(120, 180, 240)));
        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(small(mmss(elapsed), Color32::from_rgb(0, 230, 140)));
            ui.spacing_mut().slider_width = (ui.available_width() - 4.0).max(20.0);
            let slider = egui::Slider::new(&mut progress, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true);
            let resp = ui.add(slider).on_hover_text("Seek (Left/Right arrow: 5 s)");
            let seek = if resp.dragged() {
                app.scrub.drag_to(progress)
            } else if resp.drag_stopped() {
                app.scrub.release()
            } else if resp.changed() {
                app.scrub.click(progress)
            } else {
                None
            };
            if let Some(at) = seek {
                app.audio_driver
                    .seek(Duration::from_secs_f32(at * total_secs));
            }
        });
    });
}

fn transport_row(app: &mut MultiplayGuiApp, ui: &mut Ui, narrow: bool) {
    let label = |icon: &str, text: &str| {
        if narrow {
            icon.to_string()
        } else {
            format!("{icon} {text}")
        }
    };
    let is_playing = app
        .audio_driver
        .is_playing
        .load(std::sync::atomic::Ordering::Relaxed);
    ui.horizontal(|ui| {
        if ui
            .button(label("⏮", "PREV"))
            .on_hover_text("Previous (Z)")
            .clicked()
        {
            app.apply_transport(TransportAction::Previous);
        }
        let (icon, text, color) = if is_playing {
            ("⏸", "PAUSE", Color32::from_rgb(255, 180, 50))
        } else {
            ("▶", "PLAY", Color32::from_rgb(0, 230, 140))
        };
        if ui
            .button(RichText::new(label(icon, text)).strong().color(color))
            .on_hover_text(format!("{text} (space)"))
            .clicked()
        {
            app.apply_transport(TransportAction::PlayPause);
        }
        if ui
            .button(label("⏹", "STOP"))
            .on_hover_text("Stop (V)")
            .clicked()
        {
            app.apply_transport(TransportAction::Stop);
        }
        if ui
            .button(label("⏭", "NEXT"))
            .on_hover_text("Next (B)")
            .clicked()
        {
            app.apply_transport(TransportAction::Next);
        }
        ui.label(
            RichText::new("VOL")
                .size(10.0)
                .color(Color32::from_rgb(180, 200, 220)),
        );
        ui.spacing_mut().slider_width = (ui.available_width() - 4.0).max(30.0);
        let vol = egui::Slider::new(&mut app.volume, 0.0..=1.0)
            .show_value(false)
            .trailing_fill(true);
        if ui
            .add(vol)
            .on_hover_text("Volume (Up/Down arrow)")
            .changed()
        {
            app.audio_driver.set_volume(app.volume);
        }
    });
}

fn toggles_row(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(app.dj_active(), RichText::new("DJ").size(11.0))
            .on_hover_text("DJ mode (Ctrl+J)")
            .clicked()
        {
            app.toggle_dj_mode(Some(ui.ctx()));
        }
        let waiting = app.remote_requests.rows.len();
        let remote = if waiting > 0 {
            format!("📱{waiting}")
        } else {
            "📱".to_string()
        };
        for (id, text, tip) in [
            (WindowId::Equalizer, "EQ".to_string(), "Equalizer window"),
            (WindowId::Playlist, "PL".to_string(), "Playlist window"),
            (
                WindowId::Remote,
                remote,
                "Remote: phone requests and paired devices",
            ),
            (WindowId::Settings, "⚙".to_string(), "Preferences"),
        ] {
            let on = app.dock_manager.is_visible(id);
            let mut label = RichText::new(text).size(11.0);
            if id == WindowId::Remote && waiting > 0 {
                label = label.color(Color32::from_rgb(240, 180, 0)).strong();
            }
            if ui.selectable_label(on, label).on_hover_text(tip).clicked() {
                app.dock_manager.toggle_visible(id);
            }
        }
    });
}

/// Ctrl+J (Cmd+J on macOS) toggles DJ mode.
fn ctrl_j(ui: &Ui) -> bool {
    let shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::J);
    ui.ctx().input_mut(|i| i.consume_shortcut(&shortcut))
}
