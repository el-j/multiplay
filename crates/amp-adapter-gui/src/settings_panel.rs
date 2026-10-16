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
}

/// Settings panel renderer for native desktop application.
pub struct SettingsGuiPanel;

impl SettingsGuiPanel {
    pub fn show(ui: &mut Ui, active_tab: &mut GuiSettingsTab, config: &mut AppConfig) {
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
            ui.horizontal(|ui| {
                ui.selectable_value(active_tab, GuiSettingsTab::Folders, "📁 Folders");
                ui.selectable_value(active_tab, GuiSettingsTab::Audio, "🔊 Audio Output");
                ui.selectable_value(active_tab, GuiSettingsTab::Accounts, "🌐 Accounts");
                ui.selectable_value(active_tab, GuiSettingsTab::P2P, "🔗 P2P Network");
                ui.selectable_value(active_tab, GuiSettingsTab::Appearance, "🎨 Appearance");
                ui.selectable_value(active_tab, GuiSettingsTab::Plugins, "🧩 Plugins");
            });

            ui.separator();
            ui.add_space(4.0);

            match active_tab {
                GuiSettingsTab::Folders => {
                    ui.label(RichText::new("Indexed Music Directories:").strong());
                    for dir in &config.library.music_directories {
                        ui.label(format!(" • {dir}"));
                    }
                    ui.checkbox(&mut config.library.watch_filesystem, "Watch filesystem for changes");
                }
                GuiSettingsTab::Audio => {
                    ui.horizontal(|ui| {
                        ui.label("Sample Rate:");
                        ui.label(format!("{} Hz", config.audio.sample_rate));
                    });
                    ui.checkbox(&mut config.audio.loudness_normalization, "Enable EBU R128 Loudness Normalization");
                    ui.add(egui::Slider::new(&mut config.audio.crossfade_duration_ms, 0..=10000).text("Crossfade (ms)"));
                }
                GuiSettingsTab::Accounts => {
                    ui.checkbox(&mut config.streaming.enable_youtube, "Enable YouTube Music Resolver");
                    ui.checkbox(&mut config.streaming.enable_spotify, "Enable Spotify Connect");
                    ui.checkbox(&mut config.streaming.enable_soundcloud, "Enable SoundCloud Streamer");
                    ui.checkbox(&mut config.streaming.enable_bandcamp, "Enable Bandcamp Discovery");
                }
                GuiSettingsTab::P2P => {
                    ui.checkbox(&mut config.p2p.enable_p2p_mesh, "Enable Decentralized P2P Mesh");
                    ui.checkbox(&mut config.p2p.enable_soulseek, "Enable Soulseek Direct Transfer");
                    ui.add(egui::Slider::new(&mut config.p2p.max_upload_slots, 1..=16).text("Max Upload Slots"));
                }
                GuiSettingsTab::Appearance => {
                    ui.checkbox(&mut config.ui.compact_mode, "Compact Winamp Mode");
                    ui.add(egui::Slider::new(&mut config.ui.visualizer_fps, 30..=144).text("Visualizer FPS"));
                }
                GuiSettingsTab::Plugins => {
                    ui.label("No external plugins currently loaded.");
                }
            }
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
}
