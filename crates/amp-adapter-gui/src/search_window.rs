//! Universal Omnibox Search Window for GUI (`TASK-153`).
//!
//! Searches local library, YouTube, and Bandcamp in the background without blocking the UI.
//! Allows adding or playing any found track or directly pasting audio stream URLs.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::responsive::row_rtl;
use amp_domain::AudioSource;
use eframe::egui::{self, Color32, Key, RichText, Ui};

fn source_badge_style(source: &AudioSource) -> (&'static str, Color32) {
    match source {
        AudioSource::LocalFile { .. } => ("LOCAL", Color32::from_rgb(0, 200, 120)),
        AudioSource::Soulseek { .. } => ("SLSK", Color32::from_rgb(0, 180, 240)),
        AudioSource::YouTube { .. } => ("YT", Color32::from_rgb(255, 80, 80)),
        AudioSource::Bandcamp { .. } => ("BC", Color32::from_rgb(30, 180, 225)),
        AudioSource::Spotify { .. } => ("SPOTIFY", Color32::from_rgb(30, 215, 96)),
        AudioSource::P2PShared { .. } => ("P2P", Color32::from_rgb(255, 180, 40)),
        _ => ("WEB", Color32::from_rgb(180, 180, 180)),
    }
}

pub fn render_search_window_content(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = 6.0;

    // 1. Header Title
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("🌐 UNIVERSAL MUSIC SEARCH")
                .size(13.0)
                .strong()
                .color(Color32::from_rgb(0, 230, 140)),
        );
        ui.label(
            RichText::new("(YouTube & Bandcamp)")
                .size(10.0)
                .color(Color32::from_rgb(140, 160, 180)),
        );
    });

    // 2. Omnibox search input row
    let mut do_search = false;
    ui.horizontal(|ui| {
        let text_edit = egui::TextEdit::singleline(&mut app.search_query_text)
            .hint_text("🔍 Search YouTube & Bandcamp, or paste a link...")
            .desired_width(ui.available_width() - 80.0);
        let resp = ui.add(text_edit);
        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            do_search = true;
        }

        if ui
            .button(
                RichText::new("Search")
                    .strong()
                    .color(Color32::from_rgb(0, 240, 180)),
            )
            .clicked()
        {
            do_search = true;
        }
    });

    if do_search {
        app.trigger_gui_search();
    }

    // 3. Direct URL Ingestion row
    let mut do_add_url = false;
    ui.horizontal(|ui| {
        let url_edit = egui::TextEdit::singleline(&mut app.search_url_text)
            .hint_text("🔗 Paste YouTube or Bandcamp URL to add...")
            .desired_width(ui.available_width() - 80.0);
        let url_resp = ui.add(url_edit);
        if url_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            do_add_url = true;
        }

        if ui
            .button(RichText::new("+ Add URL").color(Color32::from_rgb(255, 200, 80)))
            .on_hover_text("Ingest audio directly from URL")
            .clicked()
        {
            do_add_url = true;
        }
    });

    if do_add_url && !app.search_url_text.trim().is_empty() {
        let url = app.search_url_text.clone();
        app.ingest_url(&url);
        app.search_url_text.clear();
    }

    // 4. Searching status indicator
    if app.is_searching {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new("Searching providers in background...")
                    .size(11.0)
                    .color(Color32::from_rgb(0, 200, 255)),
            );
        });
    }

    // Providers that failed (blocked, changed API, offline) say so instead of looking empty.
    if let Some(status) = &app.search_status {
        ui.label(
            RichText::new(format!("⚠ {status}"))
                .size(10.0)
                .color(Color32::from_rgb(255, 170, 80)),
        );
    }

    crate::playback_notice::notice_line(app, ui);

    ui.separator();

    // 5. Results Area
    let mut play_track = None;
    let mut add_track = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            if app.search_results.is_empty() && !app.is_searching {
                render_empty_search_prompt(ui);
                return;
            }

            for (i, track) in app.search_results.iter().enumerate() {
                let row_bg = if i % 2 == 0 {
                    Color32::from_rgb(18, 24, 32)
                } else {
                    Color32::from_rgb(14, 18, 24)
                };

                egui::Frame::NONE.fill(row_bg).show(ui, |ui| {
                    row_rtl(ui, |ui| {
                        // Actions on the right
                        if ui
                            .button(
                                RichText::new("+ Add")
                                    .size(10.0)
                                    .color(Color32::from_rgb(100, 255, 180)),
                            )
                            .on_hover_text("Add to playlist")
                            .clicked()
                        {
                            add_track = Some(track.clone());
                        }

                        if ui
                            .button(
                                RichText::new("▶ Play")
                                    .size(10.0)
                                    .color(Color32::from_rgb(0, 220, 255)),
                            )
                            .on_hover_text("Play immediately")
                            .clicked()
                        {
                            play_track = Some(track.clone());
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

                        let (badge_txt, badge_col) = source_badge_style(&track.source);
                        ui.label(RichText::new(badge_txt).size(9.0).color(badge_col));

                        // Track info on the left
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let text =
                                format!("{} - {}", track.metadata.artist, track.metadata.title);
                            ui.label(RichText::new(&text).strong().size(11.0))
                                .on_hover_text(&text);
                        });
                    });
                });
            }
        });

    if let Some(track) = play_track {
        app.play_search_result(track);
    } else if let Some(track) = add_track {
        app.add_search_result(track);
    }
}

fn render_empty_search_prompt(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(
            RichText::new("Type an artist, album, or song name above to search.")
                .size(11.0)
                .color(Color32::from_rgb(140, 160, 180)),
        );
        ui.label(
            RichText::new("Results stream in live from YouTube, Bandcamp, and local files.")
                .size(10.0)
                .color(Color32::from_rgb(100, 120, 140)),
        );
        ui.add_space(10.0);
    });
}
