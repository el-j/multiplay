//! Main eframe App state for Multiplay Native Desktop GUI.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::dj_panel::{DjConsolePanel, DjConsoleState};
use crate::equalizer_widget::EqualizerPanel;
use crate::settings_panel::{GuiSettingsTab, SettingsGuiPanel};
use crate::visualizer_widget::VisualizerWidget;
use amp_domain::source_badge::SourceKind;
use amp_domain::{AppConfig, AudioSource, Track, TrackMetadata};
use eframe::egui::{self, Color32, RichText, Ui};
use std::time::Duration;

pub struct MultiplayGuiApp {
    audio_driver: AudioPlaybackDriver,
    playlist: Vec<Track>,
    selected_track_idx: usize,
    search_query: String,
    filter_source: Option<SourceKind>,
    show_equalizer: bool,
    show_playlist: bool,
    show_dj_console: bool,
    dj_state: DjConsoleState,
    show_settings: bool,
    settings_tab: GuiSettingsTab,
    config: AppConfig,
    volume: f32,
    eq_gains: [f32; 10],
    eq_enabled: bool,
}

impl MultiplayGuiApp {
    pub fn new() -> Result<Self, String> {
        let audio_driver = AudioPlaybackDriver::new()?;

        let mut tracks = Vec::new();

        let mut t1 = TrackMetadata::new("Polynomial-C", "Aphex Twin");
        t1.duration = Some(Duration::from_secs(286));
        t1.bitrate_kbps = Some(320);
        tracks.push(Track::new(
            AudioSource::Soulseek {
                username: "peer_audiophile".to_string(),
                filename: "Aphex Twin - Polynomial-C.mp3".to_string(),
            },
            t1,
        ));

        let mut t2 = TrackMetadata::new("Music Is Math", "Boards of Canada");
        t2.duration = Some(Duration::from_secs(321));
        t2.bitrate_kbps = Some(1411);
        tracks.push(Track::new(
            AudioSource::LocalFile {
                path: "music/boc_music_is_math.flac".to_string(),
            },
            t2,
        ));

        let mut t3 = TrackMetadata::new("Computer World (Pure 24-bit)", "Kraftwerk");
        t3.duration = Some(Duration::from_secs(305));
        t3.bitrate_kbps = Some(2304);
        tracks.push(Track::new(
            AudioSource::YouTube {
                video_id: "dQw4w9WgXcQ".to_string(),
            },
            t3,
        ));

        let mut t4 = TrackMetadata::new("Aerodynamic (Lossless Remaster)", "Daft Punk");
        t4.duration = Some(Duration::from_secs(212));
        t4.bitrate_kbps = Some(1411);
        tracks.push(Track::new(
            AudioSource::P2PShared {
                peer_id: "peer_french_touch".to_string(),
                file_hash: "QmDaftPunkLosslessAudioHash123".to_string(),
            },
            t4,
        ));

        Ok(Self {
            audio_driver,
            playlist: tracks,
            selected_track_idx: 0,
            search_query: String::new(),
            filter_source: None,
            show_equalizer: true,
            show_playlist: true,
            show_dj_console: true,
            dj_state: DjConsoleState::default(),
            show_settings: false,
            settings_tab: GuiSettingsTab::default(),
            config: AppConfig::default(),
            volume: 0.85,
            eq_gains: [0.0; 10],
            eq_enabled: true,
        })
    }
}

impl eframe::App for MultiplayGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous smooth repaint for real-time visualizer animation
        ctx.request_repaint_after(Duration::from_millis(16));

        egui::CentralPanel::default().show(ctx, |ui| {
            render_main_faceplate(self, ui);

            if self.show_equalizer {
                ui.add_space(8.0);
                let prev_gains = self.eq_gains;
                let prev_enabled = self.eq_enabled;

                EqualizerPanel::new(&mut self.eq_gains, &mut self.eq_enabled).ui(ui);

                if prev_gains != self.eq_gains {
                    for (i, &g) in self.eq_gains.iter().enumerate() {
                        self.audio_driver.set_eq_band(i, g);
                    }
                }
                if prev_enabled != self.eq_enabled {
                    self.audio_driver.toggle_eq();
                }
            }

            if self.show_playlist {
                ui.add_space(8.0);
                render_playlist_panel(self, ui);
            }

            if self.show_dj_console {
                ui.add_space(8.0);
                DjConsolePanel::show(ui, &mut self.dj_state);
            }

            if self.show_settings {
                ui.add_space(8.0);
                SettingsGuiPanel::show(ui, &mut self.settings_tab, &mut self.config);
            }
        });
    }
}

fn render_main_faceplate(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    ui.group(|ui| {
        // Title Bar & VFD Display
        ui.horizontal(|ui| {
            let active_track = &app.playlist[app.selected_track_idx];
            let title_text = format!(
                "WINAMP 1.x :: {}. {} - {}",
                app.selected_track_idx + 1,
                active_track.metadata.artist,
                active_track.metadata.title
            );
            ui.heading(RichText::new(title_text).size(15.0).strong().color(Color32::from_rgb(0, 235, 136)));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("[44.1kHz / 24-bit] [ST]").size(11.0).color(Color32::from_rgb(0, 200, 240)));
            });
        });

        ui.add_space(4.0);

        // Hardware Output Device Bar
        let current_dev = app.audio_driver.current_device_name.lock().unwrap().clone();
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔊 AUDIO DEVICE:").size(10.0).strong().color(Color32::from_rgb(140, 170, 190)));
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
                if ui.selectable_label(is_selected, RichText::new(btn_text).size(10.0).color(text_color)).clicked() && !is_selected {
                    let _ = app.audio_driver.switch_device(&dev.name);
                }
            }
        });

        ui.add_space(4.0);

        // Timer & Visualizer Row
        ui.horizontal(|ui| {
            // Glowing VFD Digital Clock
            let elapsed_dur = *app.audio_driver.elapsed.lock().unwrap();
            let el_s = elapsed_dur.as_secs();
            let clock_text = format!("{:02}:{:02}", el_s / 60, el_s % 60);

            ui.vertical(|ui| {
                ui.label(RichText::new("ELAPSED TIME").size(9.0).color(Color32::from_rgb(100, 140, 160)));
                ui.label(RichText::new(clock_text).size(28.0).monospace().strong().color(Color32::from_rgb(0, 255, 140)));
            });

            ui.add_space(12.0);

            // 16-Band Real-Time Visualizer Canvas
            let spec = *app.audio_driver.visualizer_spectrum.lock().unwrap();
            VisualizerWidget::new(&spec, 46.0).ui(ui);
        });

        ui.add_space(6.0);

        // Transport Controls Row
        ui.horizontal(|ui| {
            // Previous Track
            if ui.button(RichText::new("⏮ PREV").size(12.0)).clicked() && app.selected_track_idx > 0 {
                app.selected_track_idx -= 1;
                app.audio_driver.play(app.playlist[app.selected_track_idx].clone());
            }

            // Play Button
            let is_playing = app.audio_driver.is_playing.load(std::sync::atomic::Ordering::Relaxed);
            let play_label = if is_playing { "⏸ PAUSE" } else { "▶ PLAY" };
            let play_color = if is_playing { Color32::from_rgb(255, 180, 50) } else { Color32::from_rgb(0, 230, 140) };

            if ui.button(RichText::new(play_label).size(13.0).strong().color(play_color)).clicked() {
                if !is_playing {
                    app.audio_driver.play(app.playlist[app.selected_track_idx].clone());
                } else {
                    app.audio_driver.toggle_play_pause();
                }
            }

            // Stop Button
            if ui.button(RichText::new("⏹ STOP").size(12.0)).clicked() {
                app.audio_driver.stop();
            }

            // Next Track
            if ui.button(RichText::new("⏭ NEXT").size(12.0)).clicked() && app.selected_track_idx + 1 < app.playlist.len() {
                app.selected_track_idx += 1;
                app.audio_driver.play(app.playlist[app.selected_track_idx].clone());
            }

            ui.add_space(16.0);

            // Volume Slider
            ui.label(RichText::new("VOL:").size(11.0).color(Color32::from_rgb(180, 200, 220)));
            let vol_slider = egui::Slider::new(&mut app.volume, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true);
            if ui.add_sized([100.0, 18.0], vol_slider).changed() {
                app.audio_driver.set_volume(app.volume);
            }
            ui.label(RichText::new(format!("{:02.0}%", app.volume * 100.0)).size(11.0).color(Color32::from_rgb(0, 230, 140)));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new(if app.show_settings { "SETTINGS [ON]" } else { "SETTINGS [OFF]" }).size(11.0)).clicked() {
                    app.show_settings = !app.show_settings;
                }
                if ui.button(RichText::new(if app.show_dj_console { "DJ CONSOLE [ON]" } else { "DJ CONSOLE [OFF]" }).size(11.0)).clicked() {
                    app.show_dj_console = !app.show_dj_console;
                }
                if ui.button(RichText::new(if app.show_playlist { "PLAYLIST [ON]" } else { "PLAYLIST [OFF]" }).size(11.0)).clicked() {
                    app.show_playlist = !app.show_playlist;
                }
                if ui.button(RichText::new(if app.show_equalizer { "EQUALIZER [ON]" } else { "EQUALIZER [OFF]" }).size(11.0)).clicked() {
                    app.show_equalizer = !app.show_equalizer;
                }
            });
        });
    });
}

fn render_playlist_panel(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("WINAMP PLAYLIST EDITOR").size(14.0).strong().color(Color32::from_rgb(0, 230, 140)));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::TextEdit::singleline(&mut app.search_query).hint_text("🔍 Search library...").desired_width(180.0));
            });
        });

        ui.add_space(4.0);

        // Filter Pills
        ui.horizontal(|ui| {
            if ui.selectable_label(app.filter_source.is_none(), "ALL").clicked() {
                app.filter_source = None;
            }
            if ui.selectable_label(app.filter_source == Some(SourceKind::LocalFile), "LOCAL").clicked() {
                app.filter_source = Some(SourceKind::LocalFile);
            }
            if ui.selectable_label(app.filter_source == Some(SourceKind::YouTube), "YOUTUBE").clicked() {
                app.filter_source = Some(SourceKind::YouTube);
            }
            if ui.selectable_label(app.filter_source == Some(SourceKind::Soulseek), "SOULSEEK").clicked() {
                app.filter_source = Some(SourceKind::Soulseek);
            }
            if ui.selectable_label(app.filter_source == Some(SourceKind::P2PShared), "P2P MESH").clicked() {
                app.filter_source = Some(SourceKind::P2PShared);
            }
        });

        ui.add_space(4.0);

        // Playlist Rows
        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
            for (i, track) in app.playlist.iter().enumerate() {
                let matches_query = app.search_query.is_empty()
                    || track.metadata.title.to_lowercase().contains(&app.search_query.to_lowercase())
                    || track.metadata.artist.to_lowercase().contains(&app.search_query.to_lowercase());

                let matches_filter = match app.filter_source {
                    None => true,
                    Some(SourceKind::LocalFile) => matches!(track.source, AudioSource::LocalFile { .. }),
                    Some(SourceKind::YouTube) => matches!(track.source, AudioSource::YouTube { .. }),
                    Some(SourceKind::Soulseek) => matches!(track.source, AudioSource::Soulseek { .. }),
                    Some(SourceKind::P2PShared) => matches!(track.source, AudioSource::P2PShared { .. }),
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

                egui::Frame::none().fill(row_bg).inner_margin(4.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let cursor_mark = if is_selected { "▶" } else { " " };
                        ui.label(RichText::new(cursor_mark).strong().color(Color32::from_rgb(0, 255, 140)));

                        let track_num = format!("{}.", i + 1);
                        ui.label(RichText::new(track_num).color(Color32::from_rgb(140, 160, 180)));

                        let item_text = format!("{} - {}", track.metadata.artist, track.metadata.title);
                        if ui.selectable_label(is_selected, RichText::new(item_text).strong()).clicked() {
                            app.selected_track_idx = i;
                            app.audio_driver.play(track.clone());
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let dur_str = if let Some(d) = track.metadata.duration {
                                let s = d.as_secs();
                                format!("{:02}:{:02}", s / 60, s % 60)
                            } else {
                                "--:--".to_string()
                            };
                            ui.label(RichText::new(dur_str).monospace().color(Color32::from_rgb(120, 180, 240)));

                            let (badge_txt, badge_color) = match track.source {
                                AudioSource::LocalFile { .. } => ("LOCAL", Color32::from_rgb(0, 200, 120)),
                                AudioSource::Soulseek { .. } => ("SOULSEEK", Color32::from_rgb(0, 180, 240)),
                                AudioSource::YouTube { .. } => ("YOUTUBE", Color32::from_rgb(255, 80, 80)),
                                AudioSource::P2PShared { .. } => ("P2P", Color32::from_rgb(255, 180, 40)),
                                _ => ("STREAM", Color32::from_rgb(180, 180, 180)),
                            };
                            ui.label(RichText::new(format!("[{badge_txt}]")).size(10.0).color(badge_color));
                        });
                    });
                });
            }
        });
    });
}
