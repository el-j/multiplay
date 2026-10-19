//! Categorized settings preferences modal panel for native desktop egui interface.

#![forbid(unsafe_code)]

use amp_domain::AppConfig;
use eframe::egui::{self, Color32, RichText, Ui};

/// Active preferences tab category for GUI settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GuiSettingsTab {
    #[default]
    Folders,
    Audio,
    Accounts,
    P2P,
    Appearance,
    Plugins,
    Remote,
}

/// Settings panel renderer for native desktop application.
pub struct SettingsGuiPanel;

impl SettingsGuiPanel {
    pub fn show(
        ui: &mut Ui,
        active_tab: &mut GuiSettingsTab,
        config: &mut AppConfig,
        theme: &mut amp_core_engine::WinampTheme,
        open_skin_dialog: &mut bool,
        remote: &mut crate::remote_manager::RemoteGuiManager,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new("⚙ PREFERENCES & SETTINGS")
                        .color(Color32::from_rgb(0, 235, 136))
                        .strong(),
                );
            });

            ui.add_space(4.0);

            // Tab bar
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(active_tab, GuiSettingsTab::Folders, "📁 Folders");
                ui.selectable_value(active_tab, GuiSettingsTab::Audio, "🔊 Audio Output");
                ui.selectable_value(active_tab, GuiSettingsTab::Accounts, "🌐 Accounts");
                ui.selectable_value(active_tab, GuiSettingsTab::P2P, "🔗 P2P Network");
                ui.selectable_value(active_tab, GuiSettingsTab::Appearance, "🎨 Appearance");
                ui.selectable_value(active_tab, GuiSettingsTab::Plugins, "🧩 Plugins");
                ui.selectable_value(active_tab, GuiSettingsTab::Remote, "📱 Remote Control");
            });

            ui.separator();
            ui.add_space(4.0);

            // Slider length follows the window so captioned sliders never run past the edge.
            ui.spacing_mut().slider_width = (ui.available_width() * 0.35).clamp(60.0, 220.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match active_tab {
                GuiSettingsTab::Folders => {
                    ui.label(RichText::new("Indexed Music Directories:").strong());
                    for dir in &config.library.music_directories {
                        ui.label(format!(" • {dir}"));
                    }
                    ui.checkbox(
                        &mut config.library.watch_filesystem,
                        "Watch filesystem for changes",
                    );
                }
                GuiSettingsTab::Audio => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Hardware Rate:");
                        ui.label(
                            RichText::new(format!(
                                "{} Hz (Bit-Perfect Sinc)",
                                config.audio.sample_rate
                            ))
                            .color(Color32::from_rgb(0, 230, 140)),
                        );
                    });
                    ui.checkbox(
                        &mut config.audio.loudness_normalization,
                        "Enable EBU R128 Loudness Normalization",
                    );
                    if config.audio.loudness_normalization {
                        ui.label("Target loudness (LUFS)");
                        ui.add(egui::Slider::new(&mut config.audio.target_lufs, -23..=-11));
                    }
                    ui.label("Automix crossfade in ms (0 = gapless)");
                    ui.add(egui::Slider::new(
                        &mut config.audio.crossfade_duration_ms,
                        0..=12000,
                    ));
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Binaural Headphone Crossfeed:");
                        egui::ComboBox::from_id_salt("crossfeed_cb")
                            .selected_text(&config.audio.headphone_crossfeed)
                            .show_ui(ui, |ui| {
                                for mode in &["Off", "Subtle", "Standard", "Deep"] {
                                    ui.selectable_value(
                                        &mut config.audio.headphone_crossfeed,
                                        mode.to_string(),
                                        *mode,
                                    );
                                }
                            });
                    });
                }
                GuiSettingsTab::Accounts => {
                    ui.checkbox(
                        &mut config.streaming.enable_youtube,
                        "Enable YouTube Music Resolver",
                    );
                    ui.checkbox(
                        &mut config.streaming.enable_spotify,
                        "Enable Spotify Connect",
                    );
                    ui.checkbox(
                        &mut config.streaming.enable_soundcloud,
                        "Enable SoundCloud Streamer",
                    );
                    ui.checkbox(
                        &mut config.streaming.enable_bandcamp,
                        "Enable Bandcamp Discovery",
                    );
                }
                GuiSettingsTab::P2P => {
                    ui.checkbox(
                        &mut config.p2p.enable_p2p_mesh,
                        "Enable Decentralized P2P Mesh",
                    );
                    ui.checkbox(
                        &mut config.p2p.enable_soulseek,
                        "Enable Soulseek Direct Transfer",
                    );
                    ui.add(
                        egui::Slider::new(&mut config.p2p.max_upload_slots, 1..=16)
                            .text("Max Upload Slots"),
                    );
                }
                GuiSettingsTab::Appearance => {
                    ui.label(
                        RichText::new("Winamp Classic Skin Engine (.wsz)")
                            .strong()
                            .color(Color32::from_rgb(0, 230, 140)),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Current Skin:");
                        ui.label(
                            RichText::new(&theme.name)
                                .strong()
                                .color(Color32::from_rgb(250, 204, 21)),
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Default Skin").clicked() {
                            *theme = amp_core_engine::WinampTheme::default();
                        }
                        if ui.button("Neon 98").clicked() {
                            theme.name = "Neon 98".to_string();
                            theme.accent_color = (0, 255, 200);
                            theme.vfd_fg = (0, 255, 200);
                        }
                        if ui.button("Cyberpunk Amber").clicked() {
                            theme.name = "Cyberpunk Amber".to_string();
                            theme.accent_color = (255, 175, 40);
                            theme.vfd_fg = (255, 180, 50);
                        }
                        if ui
                            .button(RichText::new("📂 Load .wsz Skin...").strong())
                            .clicked()
                        {
                            *open_skin_dialog = true;
                        }
                    });

                    ui.add_space(6.0);
                    ui.label(RichText::new("Visualizer Spectrum Gradient Preview:").size(11.0));
                    ui.horizontal_wrapped(|ui| {
                        for c in &theme.viscolor_gradient {
                            let (r, g, b) = *c;
                            let color = Color32::from_rgb(r, g, b);
                            let (rect, _) = ui.allocate_exact_size(
                                egui::Vec2::new(12.0, 12.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 1.0, color);
                        }
                    });

                    ui.add_space(4.0);
                    ui.checkbox(&mut config.ui.compact_mode, "Compact Winamp Mode");
                    let mut double = ui.ctx().zoom_factor() > 1.5;
                    if ui.checkbox(&mut double, "Double size (Ctrl+D)").changed() {
                        ui.ctx()
                            .set_zoom_factor(crate::chrome::zoom::toggled(ui.ctx().zoom_factor()));
                    }
                    ui.add(
                        egui::Slider::new(&mut config.ui.visualizer_fps, 30..=144)
                            .text("Visualizer FPS"),
                    );
                }
                GuiSettingsTab::Plugins => {
                    ui.label("No external plugins currently loaded.");
                }
                GuiSettingsTab::Remote => {
                    ui.label(
                        RichText::new("📱 SMARTPHONE REMOTE CONTROL & PAIRING")
                            .color(Color32::from_rgb(0, 235, 136))
                            .strong(),
                    );
                    ui.add_space(4.0);

                    let before = config.remote.clone();
                    ui.checkbox(&mut config.remote.enabled, "Enable remote control");
                    ui.add_enabled_ui(config.remote.enabled, |ui| {
                        ui.checkbox(
                            &mut config.remote.allow_lan,
                            "Allow devices on my network (phones on the same Wi-Fi)",
                        );
                    });
                    if config.remote.allow_lan {
                        ui.label(
                            RichText::new(
                                "Anyone on this network who scans the QR code can control playback. \
                                 Use only on networks you trust.",
                            )
                            .size(10.0)
                            .color(Color32::from_rgb(240, 180, 0)),
                        );
                    }
                    if config.remote != before {
                        remote.apply_config(&config.remote);
                    }

                    if let Some(err) = &remote.last_error {
                        ui.colored_label(Color32::from_rgb(240, 80, 80), err);
                    }

                    ui.add_space(4.0);
                    if !remote.is_running {
                        ui.label("Remote control is off. Nothing is listening for connections.");
                        return;
                    }

                    ui.horizontal_wrapped(|ui| {
                        let scope = if remote.allow_lan { "network" } else { "this computer only" };
                        ui.label(format!("Listening on port {} ({scope})", remote.port));
                        ui.separator();
                        ui.label(format!(
                            "Connected: {} · Paired devices: {}",
                            remote.active_peers_count(),
                            remote.paired_devices()
                        ));
                    });

                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if let Some(ref matrix) = remote.qr_matrix {
                            crate::qr_widget::draw_qr_code(ui, matrix, 140.0);
                        }

                        ui.vertical(|ui| {
                            ui.label("Scan the QR code with your phone camera to pair.");
                            ui.label("Each code works once and expires after 10 minutes.");
                            ui.add_space(4.0);
                            ui.label(format!("Token: {}", remote.current_token));
                            if ui.button("🔄 New pairing code").clicked() {
                                remote.refresh_token();
                            }
                            if ui.button("🚫 Forget all paired devices").clicked() {
                                remote.revoke_all_sessions();
                            }
                            ui.add_space(4.0);
                            ui.label("Direct URL:");
                            let mut url = remote.pairing_url();
                            ui.text_edit_singleline(&mut url);
                        });
                    });
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_panel_tabs() {
        let mut tab = GuiSettingsTab::default();
        assert_eq!(tab, GuiSettingsTab::Folders);

        tab = GuiSettingsTab::Audio;
        assert_eq!(tab, GuiSettingsTab::Audio);
    }

    #[test]
    fn test_settings_appearance_theme() {
        let tab = GuiSettingsTab::Appearance;
        let config = AppConfig::default();
        let theme = amp_core_engine::WinampTheme::default();
        let open_dialog = false;

        assert_eq!(tab, GuiSettingsTab::Appearance);
        assert_eq!(config.ui.visualizer_fps, 60);
        assert_eq!(theme.name, "Classic Winamp 1.x");
        assert_eq!(theme.viscolor_gradient[0], (255, 0, 0));
        assert!(!open_dialog);
    }

    #[test]
    fn test_settings_remote_tab() {
        let tab = GuiSettingsTab::Remote;
        assert_eq!(tab, GuiSettingsTab::Remote);
    }
}
