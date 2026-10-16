//! Categorized preferences and configuration settings modal for Ratatui TUI.

#![forbid(unsafe_code)]

use amp_domain::AppConfig;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// Categories for preferences configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    MusicFolders,
    AudioHardware,
    OnlineAccounts,
    P2PNetwork,
    Appearance,
    PluginOptions,
}

impl SettingsCategory {
    pub const ALL: [Self; 6] = [
        Self::MusicFolders,
        Self::AudioHardware,
        Self::OnlineAccounts,
        Self::P2PNetwork,
        Self::Appearance,
        Self::PluginOptions,
    ];

    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::MusicFolders => "Music Folders",
            Self::AudioHardware => "Audio Hardware",
            Self::OnlineAccounts => "Online Accounts",
            Self::P2PNetwork => "P2P Network",
            Self::Appearance => "Appearance & Theme",
            Self::PluginOptions => "Plugins",
        }
    }
}

/// State container for the TUI settings view modal.
#[derive(Debug, Clone)]
pub struct SettingsViewState {
    pub active_category: SettingsCategory,
    pub config: AppConfig,
}

impl Default for SettingsViewState {
    fn default() -> Self {
        Self {
            active_category: SettingsCategory::MusicFolders,
            config: AppConfig::default(),
        }
    }
}

impl SettingsViewState {
    pub fn next_category(&mut self) {
        let current_idx = SettingsCategory::ALL.iter().position(|&c| c == self.active_category).unwrap_or(0);
        let next_idx = (current_idx + 1) % SettingsCategory::ALL.len();
        self.active_category = SettingsCategory::ALL[next_idx];
    }

    pub fn prev_category(&mut self) {
        let current_idx = SettingsCategory::ALL.iter().position(|&c| c == self.active_category).unwrap_or(0);
        let prev_idx = if current_idx == 0 { SettingsCategory::ALL.len() - 1 } else { current_idx - 1 };
        self.active_category = SettingsCategory::ALL[prev_idx];
    }
}

/// Two-pane settings preferences modal widget.
pub struct SettingsViewModal<'a> {
    state: &'a SettingsViewState,
}

impl<'a> SettingsViewModal<'a> {
    #[must_use]
    pub fn new(state: &'a SettingsViewState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for SettingsViewModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 50 || area.height < 10 {
            return;
        }

        let bg_style = Style::default().bg(Color::Rgb(16, 20, 26)).fg(Color::White);
        let header_style = Style::default()
            .bg(Color::Rgb(24, 32, 42))
            .fg(Color::Rgb(0, 235, 136))
            .add_modifier(Modifier::BOLD);
        let active_cat_style = Style::default()
            .bg(Color::Rgb(0, 160, 90))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let idle_cat_style = Style::default().fg(Color::Rgb(160, 180, 200));
        let border_style = Style::default().fg(Color::Rgb(60, 80, 100));

        // Background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_style(bg_style);
            }
        }

        // Header
        let title = " ═══ MULTIPLAY PREFERENCES ═════════════════════════════════════════════════";
        buf.set_string(area.left(), area.top(), title, border_style);
        buf.set_string(area.left() + 5, area.top(), "MULTIPLAY PREFERENCES", header_style);

        // Sidebar width
        let sidebar_width = 22;
        let content_left = area.left() + sidebar_width + 1;

        // Render Sidebar Categories
        for (i, &cat) in SettingsCategory::ALL.iter().enumerate() {
            let y = area.top() + 2 + (i as u16);
            if y < area.bottom() - 1 {
                let style = if cat == self.state.active_category {
                    active_cat_style
                } else {
                    idle_cat_style
                };
                let label = format!(" {:<18}", cat.label());
                buf.set_string(area.left() + 1, y, &label, style);
            }
        }

        // Vertical divider
        for y in (area.top() + 1)..(area.bottom() - 1) {
            buf.set_string(area.left() + sidebar_width, y, "│", border_style);
        }

        // Render Active Category Details on right pane
        let content_y = area.top() + 2;
        match self.state.active_category {
            SettingsCategory::MusicFolders => {
                buf.set_string(content_left + 1, content_y, "[ Indexed Music Directories ]", header_style);
                for (idx, dir) in self.state.config.library.music_directories.iter().enumerate() {
                    let line = format!(" • {dir}");
                    buf.set_string(content_left + 1, content_y + 2 + idx as u16, &line, idle_cat_style);
                }
            }
            SettingsCategory::AudioHardware => {
                buf.set_string(content_left + 1, content_y, "[ Audio Hardware Output ]", header_style);
                let sr = format!("Sample Rate: {} Hz", self.state.config.audio.sample_rate);
                let buf_sz = format!("Buffer Size: {} frames", self.state.config.audio.buffer_frames);
                let ram = format!("Precache RAM: {} MB", self.state.config.audio.precache_ram_mb);
                buf.set_string(content_left + 1, content_y + 2, &sr, idle_cat_style);
                buf.set_string(content_left + 1, content_y + 3, &buf_sz, idle_cat_style);
                buf.set_string(content_left + 1, content_y + 4, &ram, idle_cat_style);
            }
            SettingsCategory::OnlineAccounts => {
                buf.set_string(content_left + 1, content_y, "[ Streaming Providers ]", header_style);
                let yt = format!("YouTube: {}", if self.state.config.streaming.enable_youtube { "ENABLED" } else { "DISABLED" });
                let sp = format!("Spotify: {}", if self.state.config.streaming.enable_spotify { "ENABLED" } else { "DISABLED" });
                let sc = format!("SoundCloud: {}", if self.state.config.streaming.enable_soundcloud { "ENABLED" } else { "DISABLED" });
                buf.set_string(content_left + 1, content_y + 2, &yt, idle_cat_style);
                buf.set_string(content_left + 1, content_y + 3, &sp, idle_cat_style);
                buf.set_string(content_left + 1, content_y + 4, &sc, idle_cat_style);
            }
            SettingsCategory::P2PNetwork => {
                buf.set_string(content_left + 1, content_y, "[ P2P Mesh & Soulseek Network ]", header_style);
                let mesh = format!("Mesh Listener Port: {}", self.state.config.p2p.mesh_listen_port);
                let slsk = format!("Soulseek Port: {}", self.state.config.p2p.soulseek_listen_port);
                buf.set_string(content_left + 1, content_y + 2, &mesh, idle_cat_style);
                buf.set_string(content_left + 1, content_y + 3, &slsk, idle_cat_style);
            }
            SettingsCategory::Appearance => {
                buf.set_string(content_left + 1, content_y, "[ Appearance & Skins ]", header_style);
                let fps = format!("Visualizer Target FPS: {}", self.state.config.ui.visualizer_fps);
                let snap = format!("Magnetic Snap Distance: {} cells", self.state.config.ui.magnetic_snap_distance);
                buf.set_string(content_left + 1, content_y + 2, &fps, idle_cat_style);
                buf.set_string(content_left + 1, content_y + 3, &snap, idle_cat_style);
            }
            SettingsCategory::PluginOptions => {
                buf.set_string(content_left + 1, content_y, "[ Installed Audio Plugins ]", header_style);
                buf.set_string(content_left + 1, content_y + 2, "No external plugins configured.", idle_cat_style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_view_category_navigation() {
        let mut state = SettingsViewState::default();
        assert_eq!(state.active_category, SettingsCategory::MusicFolders);

        state.next_category();
        assert_eq!(state.active_category, SettingsCategory::AudioHardware);

        state.prev_category();
        assert_eq!(state.active_category, SettingsCategory::MusicFolders);

        // Render check
        let modal = SettingsViewModal::new(&state);
        let area = Rect::new(0, 0, 80, 20);
        let mut buffer = Buffer::empty(area);
        modal.render(area, &mut buffer);

        let mut found_header = false;
        for y in 0..20 {
            let mut line = String::new();
            for x in 0..80 {
                line.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            if line.contains("MULTIPLAY PREFERENCES") {
                found_header = true;
            }
        }
        assert!(found_header, "Preferences header must be rendered");
    }
}
