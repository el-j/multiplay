//! Winamp-style modular playlist editor window with drag-and-drop ingestion and file actions.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use crate::responsive::row_rtl;
use amp_domain::source_badge::SourceKind;
use amp_domain::AudioSource;
use eframe::egui::{self, Color32, RichText, Ui};

/// Below this width the toolbar shows icons only and rows hide the source badge.
const NARROW: f32 = 460.0;
const SOURCES: [(Option<SourceKind>, &str); 6] = [
    (None, "All sources"),
    (Some(SourceKind::LocalFile), "Local"),
    (Some(SourceKind::YouTube), "YouTube"),
    (Some(SourceKind::Bandcamp), "Bandcamp"),
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

    crate::playback_notice::notice_line(app, ui);

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
            if ui
                .button(label("🔍", "Search"))
                .on_hover_text("Open Universal Search & Add (Ctrl+K)")
                .clicked()
            {
                app.dock_manager
                    .toggle_visible(crate::docking::WindowId::Search);
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
    egui::Frame::NONE
        .fill(Color32::from_rgb(18, 24, 32))
        .stroke(egui::Stroke::new(1.0f32, Color32::from_rgb(0, 200, 120)))
        .corner_radius(4.0)
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
        Some(SourceKind::Bandcamp) => matches!(track.source, AudioSource::Bandcamp { .. }),
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
        AudioSource::Bandcamp { .. } => ("BC", Color32::from_rgb(30, 180, 225)),
        AudioSource::P2PShared { .. } => ("P2P", Color32::from_rgb(255, 180, 40)),
        _ => ("WEB", Color32::from_rgb(180, 180, 180)),
    }
}

fn render_track_rows(app: &mut MultiplayGuiApp, ui: &mut Ui, narrow: bool) {
    let mut track_to_remove = None;
    let mut track_to_play = None;
    let mut cycle_assign = None;
    let dj = app.dj_active();
    if dj {
        assign_keys(app, ui);
    }
    let order: Vec<amp_domain::TrackId> = app.playlist.iter().map(|t| t.id).collect();
    let next = if dj {
        amp_domain::next_markers(&order, &app.deck_assign, &app.dj_played)
    } else {
        [None, None]
    };

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
        egui::Frame::NONE.fill(row_bg).show(ui, |ui| {
            row_rtl(ui, |ui| {
                let remove = RichText::new("🗙")
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
                    if let Some(who) = app.requested_by.get(&track.id) {
                        ui.label(RichText::new("📱").size(9.0))
                            .on_hover_text(format!("Requested by {who}"));
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if dj {
                        let a = app.deck_assign.get(&track.id).copied().unwrap_or_default();
                        if deck_badge(ui, a, app.dj_played.contains(&track.id)).clicked() {
                            cycle_assign = Some(track.id);
                        }
                        for (deck, color) in [(0, DECK_A), (1, DECK_B)] {
                            if next[deck] == Some(i) {
                                let tag = ["⏵A", "⏵B"][deck];
                                ui.label(RichText::new(tag).size(9.0).strong().color(color))
                                    .on_hover_text(format!("Next on deck {}", ["A", "B"][deck]));
                            }
                        }
                    }
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

    if let Some(id) = cycle_assign {
        let a = app.deck_assign.entry(id).or_default();
        *a = a.cycle();
        app.save_session();
    }

    if let (Some(idx), true) = (track_to_play, dj) {
        // In DJ mode a click selects; tracks reach the decks by assignment or the load buttons.
        app.selected_track_idx = idx;
    } else if let Some(idx) = track_to_play {
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

const DECK_A: Color32 = Color32::from_rgb(0, 200, 255);
const DECK_B: Color32 = Color32::from_rgb(255, 150, 40);

/// The DJ deck badge of a row; click to cycle none → A → B → both.
fn deck_badge(ui: &mut Ui, a: amp_domain::DeckAssignment, played: bool) -> egui::Response {
    use amp_domain::DeckAssignment as D;
    let color = match a {
        D::None => Color32::from_rgb(90, 100, 115),
        D::A => DECK_A,
        D::B => DECK_B,
        D::Both => Color32::from_rgb(200, 210, 120),
    };
    let color = if played {
        color.gamma_multiply(0.45)
    } else {
        color
    };
    let text = RichText::new(format!("{:>2}", a.label()))
        .monospace()
        .size(10.0)
        .strong()
        .color(color);
    ui.add(egui::Button::new(text).small().frame(false))
        .on_hover_text("Deck: click to cycle A / B / both / none ([ ] \\ keys)")
}

/// `[` assigns the selected row to deck A, `]` to deck B, `\\` to both (again: none).
fn assign_keys(app: &mut MultiplayGuiApp, ui: &Ui) {
    use amp_domain::DeckAssignment as D;
    let pressed = ui.ctx().input(|i| {
        [
            (egui::Key::OpenBracket, D::A),
            (egui::Key::CloseBracket, D::B),
            (egui::Key::Backslash, D::Both),
        ]
        .into_iter()
        .find(|(k, _)| i.key_pressed(*k))
        .map(|(_, a)| a)
    });
    let (Some(want), Some(track)) = (pressed, app.playlist.get(app.selected_track_idx)) else {
        return;
    };
    let slot = app.deck_assign.entry(track.id).or_default();
    *slot = if *slot == want { D::None } else { want };
    app.save_session();
}
