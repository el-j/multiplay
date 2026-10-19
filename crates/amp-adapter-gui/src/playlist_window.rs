//! Winamp-style modular playlist editor window with drag-and-drop ingestion and file actions.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use crate::responsive::row_rtl;
use amp_domain::source_badge::SourceKind;
use amp_domain::AudioSource;
use eframe::egui::{self, Color32, RichText, Ui};

/// Below this width the toolbar shows icons only and rows hide the source badge.
const NARROW: f32 = 460.0;
const SOURCES: [(Option<SourceKind>, &str); 5] = [
    (None, "All sources"),
    (Some(SourceKind::LocalFile), "Local"),
    (Some(SourceKind::YouTube), "YouTube"),
    (Some(SourceKind::Soulseek), "Soulseek"),
    (Some(SourceKind::P2PShared), "P2P"),
];

pub fn render_playlist_window_content(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = 4.0;
    let narrow = ui.available_width() < NARROW;

    // 1. Title & search
    row_rtl(ui, |ui| {
        let width = (ui.available_width() * 0.5).clamp(90.0, 220.0);
        ui.add(
            egui::TextEdit::singleline(&mut app.search_query)
                .hint_text("🔍 Search")
                .desired_width(width),
        );
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new("PLAYLIST")
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(0, 230, 140)),
                )
                .truncate(),
            );
        });
    });

    // 2. Actions & source filter
    row_rtl(ui, |ui| {
        let current = SOURCES
            .iter()
            .find(|(k, _)| *k == app.filter_source)
            .map_or("All sources", |(_, n)| *n);
        egui::ComboBox::from_id_salt("playlist_source")
            .selected_text(current)
            .width(if narrow { 80.0 } else { 110.0 })
            .show_ui(ui, |ui| {
                for (kind, name) in SOURCES {
                    ui.selectable_value(&mut app.filter_source, kind, name);
                }
            });
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let label = |icon: &str, text: &str| {
                if narrow {
                    icon.to_string()
                } else {
                    format!("{icon} {text}")
                }
            };
            if ui
                .button(label("📁", "Files"))
                .on_hover_text("Add files")
                .clicked()
            {
                app.open_file_dialog();
            }
            if ui
                .button(label("📂", "Folder"))
                .on_hover_text("Add folder")
                .clicked()
            {
                app.open_folder_dialog();
            }
            if ui
                .button(label("🎵", "Scan"))
                .on_hover_text("Scan ~/Music")
                .clicked()
            {
                app.scan_user_music_dir();
            }
            let clear = RichText::new(label("🗑", "Clear")).color(Color32::from_rgb(255, 100, 100));
            if ui.button(clear).on_hover_text("Clear playlist").clicked() {
                app.playlist.clear();
                app.selected_track_idx = 0;
                app.audio_driver.stop();
                app.save_session();
            }
        });
    });

    // 3. Track list or empty drop zone; takes all remaining height and scrolls.
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            if app.playlist.is_empty() {
                render_empty_playlist_drop_zone(app, ui);
            } else {
                render_track_rows(app, ui, narrow);
            }
        });
}

fn render_empty_playlist_drop_zone(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    egui::Frame::none()
        .fill(Color32::from_rgb(18, 24, 32))
        .stroke(egui::Stroke::new(1.0f32, Color32::from_rgb(0, 200, 120)))
        .rounding(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("🎧 Drop audio files or folders here")
                        .strong()
                        .color(Color32::from_rgb(0, 235, 136)),
                );
                ui.label(
                    RichText::new("mp3 · flac · wav · ogg · m4a")
                        .size(10.0)
                        .color(Color32::from_rgb(170, 190, 210)),
                );
            });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(RichText::new("📁 Open files…").color(Color32::from_rgb(0, 240, 180)))
                    .clicked()
                {
                    app.open_file_dialog();
                }
                if ui
                    .button(RichText::new("🎵 Scan ~/Music").color(Color32::from_rgb(255, 180, 50)))
                    .clicked()
                {
                    app.scan_user_music_dir();
                }
            });
        });
}

fn track_matches(app: &MultiplayGuiApp, track: &amp_domain::Track) -> bool {
    let q = app.search_query.to_lowercase();
    let matches_query = q.is_empty()
        || track.metadata.title.to_lowercase().contains(&q)
        || track.metadata.artist.to_lowercase().contains(&q);
    let matches_filter = match app.filter_source {
        None => true,
        Some(SourceKind::LocalFile) => matches!(track.source, AudioSource::LocalFile { .. }),
        Some(SourceKind::YouTube) => matches!(track.source, AudioSource::YouTube { .. }),
        Some(SourceKind::Soulseek) => matches!(track.source, AudioSource::Soulseek { .. }),
        Some(SourceKind::P2PShared) => matches!(track.source, AudioSource::P2PShared { .. }),
        _ => true,
    };
    matches_query && matches_filter
}

fn badge(source: &AudioSource) -> (&'static str, Color32) {
    match source {
        AudioSource::LocalFile { .. } => ("LOCAL", Color32::from_rgb(0, 200, 120)),
        AudioSource::Soulseek { .. } => ("SLSK", Color32::from_rgb(0, 180, 240)),
        AudioSource::YouTube { .. } => ("YT", Color32::from_rgb(255, 80, 80)),
        AudioSource::P2PShared { .. } => ("P2P", Color32::from_rgb(255, 180, 40)),
        _ => ("WEB", Color32::from_rgb(180, 180, 180)),
    }
}

fn render_track_rows(app: &mut MultiplayGuiApp, ui: &mut Ui, narrow: bool) {
    let mut track_to_remove = None;
    let mut track_to_play = None;

    for (i, track) in app.playlist.iter().enumerate() {
        if !track_matches(app, track) {
            continue;
        }
        let is_selected = i == app.selected_track_idx;
        let row_bg = if is_selected {
            Color32::from_rgb(0, 60, 110)
        } else if i % 2 == 0 {
            Color32::from_rgb(16, 22, 28)
        } else {
            Color32::from_rgb(12, 16, 22)
        };
        egui::Frame::none().fill(row_bg).show(ui, |ui| {
            row_rtl(ui, |ui| {
                let remove = RichText::new("✕")
                    .size(10.0)
                    .color(Color32::from_rgb(220, 80, 80));
                if ui.small_button(remove).on_hover_text("Remove").clicked() {
                    track_to_remove = Some(i);
                }
                let dur = track.metadata.duration.map_or("--:--".to_string(), |d| {
                    format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60)
                });
                ui.label(
                    RichText::new(dur)
                        .monospace()
                        .size(10.0)
                        .color(Color32::from_rgb(120, 180, 240)),
                );
                if !narrow {
                    let (txt, color) = badge(&track.source);
                    ui.label(RichText::new(txt).size(9.0).color(color));
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let marker = if is_selected { "▶" } else { " " };
                    ui.label(RichText::new(marker).color(Color32::from_rgb(0, 255, 140)));
                    ui.label(
                        RichText::new(format!("{}.", i + 1))
                            .color(Color32::from_rgb(140, 160, 180)),
                    );
                    let text = format!("{} - {}", track.metadata.artist, track.metadata.title);
                    if ui
                        .selectable_label(is_selected, RichText::new(&text).strong())
                        .on_hover_text(text)
                        .clicked()
                    {
                        track_to_play = Some(i);
                    }
                });
            });
        });
    }

    if let Some(idx) = track_to_play {
        app.selected_track_idx = idx;
        app.audio_driver.play(app.playlist[idx].clone());
        let upcoming = app.playlist.get(idx + 1).cloned();
        app.audio_driver.set_next_track(upcoming);
    }

    if let Some(idx) = track_to_remove {
        if idx < app.playlist.len() {
            app.playlist.remove(idx);
            if app.selected_track_idx >= app.playlist.len() && !app.playlist.is_empty() {
                app.selected_track_idx = app.playlist.len() - 1;
            }
            app.save_session();
        }
    }
}
