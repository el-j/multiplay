//! Winamp-style modular playlist editor window with drag-and-drop ingestion and file actions.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use amp_domain::source_badge::SourceKind;
use amp_domain::AudioSource;
use eframe::egui::{self, Color32, RichText, Ui};

pub fn render_playlist_window_content(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    ui.group(|ui| {
        // 1. Title Bar & Search
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("WINAMP PLAYLIST EDITOR")
                    .size(14.0)
                    .strong()
                    .color(Color32::from_rgb(0, 230, 140)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.search_query)
                        .hint_text("🔍 Search playlist...")
                        .desired_width(170.0),
                );
            });
        });

        ui.add_space(4.0);

        // 2. Action Toolbar & Filter Pills
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("📁 Add Files").size(11.0))
                .clicked()
            {
                app.open_file_dialog();
            }
            if ui
                .button(RichText::new("📂 Add Folder").size(11.0))
                .clicked()
            {
                app.open_folder_dialog();
            }
            if ui
                .button(RichText::new("🎵 Scan ~/Music").size(11.0))
                .clicked()
            {
                app.scan_user_music_dir();
            }
            if ui
                .button(
                    RichText::new("🗑 Clear")
                        .size(11.0)
                        .color(Color32::from_rgb(255, 100, 100)),
                )
                .clicked()
            {
                app.playlist.clear();
                app.selected_track_idx = 0;
                app.audio_driver.stop();
                app.save_session();
            }

            ui.separator();

            if ui
                .selectable_label(app.filter_source.is_none(), "ALL")
                .clicked()
            {
                app.filter_source = None;
            }
            if ui
                .selectable_label(app.filter_source == Some(SourceKind::LocalFile), "LOCAL")
                .clicked()
            {
                app.filter_source = Some(SourceKind::LocalFile);
            }
            if ui
                .selectable_label(app.filter_source == Some(SourceKind::YouTube), "YOUTUBE")
                .clicked()
            {
                app.filter_source = Some(SourceKind::YouTube);
            }
            if ui
                .selectable_label(app.filter_source == Some(SourceKind::Soulseek), "SOULSEEK")
                .clicked()
            {
                app.filter_source = Some(SourceKind::Soulseek);
            }
            if ui
                .selectable_label(app.filter_source == Some(SourceKind::P2PShared), "P2P")
                .clicked()
            {
                app.filter_source = Some(SourceKind::P2PShared);
            }
        });

        ui.add_space(6.0);

        // 3. Track List or Empty Onboarding Drop Zone
        if app.playlist.is_empty() {
            render_empty_playlist_drop_zone(app, ui);
        } else {
            render_track_rows(app, ui);
        }
    });
}

fn render_empty_playlist_drop_zone(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    egui::Frame::none()
        .fill(Color32::from_rgb(18, 24, 32))
        .stroke(egui::Stroke::new(1.5f32, Color32::from_rgb(0, 200, 120)))
        .rounding(4.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(RichText::new("🎧 MULTIPLAY WINAMP AUDIO ENGINE").size(15.0).strong().color(Color32::from_rgb(0, 235, 136)));
                ui.add_space(4.0);
                ui.label(RichText::new("Drop your audio files (.mp3, .flac, .wav, .ogg, .m4a) or stream links here").size(12.0).color(Color32::from_rgb(170, 190, 210)));
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button(RichText::new("📁 Open Files...").size(12.0).strong().color(Color32::from_rgb(0, 240, 180))).clicked() {
                        app.open_file_dialog();
                    }
                    if ui.button(RichText::new("🎵 Scan ~/Music Folder").size(12.0).color(Color32::from_rgb(255, 180, 50))).clicked() {
                        app.scan_user_music_dir();
                    }
                });
                ui.add_space(8.0);
            });
        });
}

fn render_track_rows(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let mut track_to_remove = None;
    let mut track_to_play = None;

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            for (i, track) in app.playlist.iter().enumerate() {
                let matches_query = app.search_query.is_empty()
                    || track
                        .metadata
                        .title
                        .to_lowercase()
                        .contains(&app.search_query.to_lowercase())
                    || track
                        .metadata
                        .artist
                        .to_lowercase()
                        .contains(&app.search_query.to_lowercase());

                let matches_filter = match app.filter_source {
                    None => true,
                    Some(SourceKind::LocalFile) => {
                        matches!(track.source, AudioSource::LocalFile { .. })
                    }
                    Some(SourceKind::YouTube) => {
                        matches!(track.source, AudioSource::YouTube { .. })
                    }
                    Some(SourceKind::Soulseek) => {
                        matches!(track.source, AudioSource::Soulseek { .. })
                    }
                    Some(SourceKind::P2PShared) => {
                        matches!(track.source, AudioSource::P2PShared { .. })
                    }
                    _ => true,
                };

                if !matches_query || !matches_filter {
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

                egui::Frame::none()
                    .fill(row_bg)
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let cursor_mark = if is_selected { "▶" } else { " " };
                            ui.label(
                                RichText::new(cursor_mark)
                                    .strong()
                                    .color(Color32::from_rgb(0, 255, 140)),
                            );

                            let track_num = format!("{}.", i + 1);
                            ui.label(
                                RichText::new(track_num).color(Color32::from_rgb(140, 160, 180)),
                            );

                            let item_text =
                                format!("{} - {}", track.metadata.artist, track.metadata.title);
                            if ui
                                .selectable_label(is_selected, RichText::new(item_text).strong())
                                .clicked()
                            {
                                track_to_play = Some(i);
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button(
                                            RichText::new("✕")
                                                .size(10.0)
                                                .color(Color32::from_rgb(220, 80, 80)),
                                        )
                                        .clicked()
                                    {
                                        track_to_remove = Some(i);
                                    }

                                    let dur_str = if let Some(d) = track.metadata.duration {
                                        let s = d.as_secs();
                                        format!("{:02}:{:02}", s / 60, s % 60)
                                    } else {
                                        "--:--".to_string()
                                    };
                                    ui.label(
                                        RichText::new(dur_str)
                                            .monospace()
                                            .color(Color32::from_rgb(120, 180, 240)),
                                    );

                                    let (badge_txt, badge_color) = match track.source {
                                        AudioSource::LocalFile { .. } => {
                                            ("LOCAL", Color32::from_rgb(0, 200, 120))
                                        }
                                        AudioSource::Soulseek { .. } => {
                                            ("SOULSEEK", Color32::from_rgb(0, 180, 240))
                                        }
                                        AudioSource::YouTube { .. } => {
                                            ("YOUTUBE", Color32::from_rgb(255, 80, 80))
                                        }
                                        AudioSource::P2PShared { .. } => {
                                            ("P2P", Color32::from_rgb(255, 180, 40))
                                        }
                                        _ => ("STREAM", Color32::from_rgb(180, 180, 180)),
                                    };
                                    ui.label(
                                        RichText::new(format!("[{badge_txt}]"))
                                            .size(10.0)
                                            .color(badge_color),
                                    );
                                },
                            );
                        });
                    });
            }
        });

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
