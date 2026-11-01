//! Categorized settings preferences modal panel for native desktop egui interface.

#![forbid(unsafe_code)]

use amp_domain::AppConfig;
use eframe::egui::{self, Color32, RichText, Ui};

/// Windows and dialogs the settings panel asks the app to open after drawing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SettingsOpens {
    pub skin_dialog: bool,
    pub remote_window: bool,
    /// A button in the About tab.
    pub about: crate::about::AboutAction,
}

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
    About,
}

/// Output devices for the audio tab.
#[derive(Debug, Clone, Copy)]
pub struct OutputDevices<'a> {
    /// Device names.
    pub names: &'a [String],
    /// Why the chosen DJ headphone device could not be used.
    pub headphones_error: Option<&'a str>,
}

/// Settings panel renderer for native desktop application.
pub struct SettingsGuiPanel;

impl SettingsGuiPanel {
    pub fn show(
        ui: &mut Ui,
        active_tab: &mut GuiSettingsTab,
        config: &mut AppConfig,
        theme: &mut amp_core_engine::WinampTheme,
        opens: &mut SettingsOpens,
        remote: &mut crate::remote_manager::RemoteGuiManager,
        outputs: OutputDevices<'_>,
    ) -> bool {
        let (devices, headphones_error) = (outputs.names, outputs.headphones_error);
        let mut done = false;
        ui.group(|ui| {
            crate::responsive::row_rtl(ui, |ui| {
                done = ui
                    .button(RichText::new("Done").strong())
                    .on_hover_text("Close preferences (Esc)")
                    .clicked();
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new("⚙ PREFERENCES")
                                .size(14.0)
                                .color(Color32::from_rgb(0, 235, 136))
                                .strong(),
                        )
                        .truncate(),
                    );
                });
            });

            ui.add_space(4.0);

            // Tab bar
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(active_tab, GuiSettingsTab::Folders, "📁 Folders");
                ui.selectable_value(active_tab, GuiSettingsTab::Audio, "🔊 Audio Output");
                ui.selectable_value(active_tab, GuiSettingsTab::Accounts, "🌐 Accounts");
                ui.selectable_value(active_tab, GuiSettingsTab::P2P, "🔗 P2P Network");
                ui.selectable_value(active_tab, GuiSettingsTab::Appearance, "🎨 Appearance");
                ui.selectable_value(active_tab, GuiSettingsTab::Plugins, "🔌 Plugins");
                ui.selectable_value(active_tab, GuiSettingsTab::Remote, "📱 Remote Control");
                ui.selectable_value(active_tab, GuiSettingsTab::About, "ℹ About");
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
                        ui.label("DJ headphones (cue):");
                        let current = config
                            .audio
                            .dj_headphones_device
                            .clone()
                            .unwrap_or_else(|| "None".into());
                        egui::ComboBox::from_id_salt("dj_phones_cb")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(config.audio.dj_headphones_device.is_none(), "None").clicked() {
                                    config.audio.dj_headphones_device = None;
                                }
                                for d in devices {
                                    let on = config.audio.dj_headphones_device.as_deref() == Some(d);
                                    if ui.selectable_label(on, d).clicked() {
                                        config.audio.dj_headphones_device = Some(d.clone());
                                    }
                                }
                            });
                    });
                    if let Some(err) = headphones_error {
                        ui.colored_label(Color32::from_rgb(240, 160, 60), err);
                    }
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
                        RichText::new("Skins (classic Winamp® .wsz format)")
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
                            opens.skin_dialog = true;
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
                    ui.checkbox(&mut config.ui.compact_mode, "Compact mode");
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
                GuiSettingsTab::About => crate::about::show(ui, &mut opens.about, &mut config.updates),
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

                    let scope = if remote.allow_lan { "your network" } else { "this computer only" };
                    ui.label(format!("Listening on port {} ({scope})", remote.port));
                    ui.label(format!(
                        "Connected: {} · Paired devices: {}",
                        remote.active_peers_count(),
                        remote.paired_devices()
                    ));
                    ui.add_space(4.0);
                    ui.label("Pairing, paired devices and phone requests are in the Remote window.");
                    if ui.button("📱 Open Remote window").clicked() {
                        opens.remote_window = true;
                    }
                }
            });
        });
        done
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
        assert_eq!(theme.name, "multiplay Classic");
        assert_eq!(theme.viscolor_gradient[0], (255, 0, 0));
        assert!(!open_dialog);
    }

    #[test]
    fn test_settings_remote_tab() {
        let tab = GuiSettingsTab::Remote;
        assert_eq!(tab, GuiSettingsTab::Remote);
    }
}
