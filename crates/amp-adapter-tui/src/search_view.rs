//! Omnibar Search Modal with Dynamic Source Filter Pills and Badges.
//! Formats multi-source query results with retro badge icons, colors, and keybindings.

#![forbid(unsafe_code)]

use amp_domain::source_badge::{SourceFilterMask, SourceKind};
use amp_domain::Track;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct SearchViewState {
    pub is_open: bool,
    pub input_query: String,
    pub filter_mask: SourceFilterMask,
    pub active_pill_idx: usize,
    pub results: Vec<Track>,
    pub selected_result_idx: usize,
}

impl SearchViewState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            input_query: String::new(),
            filter_mask: SourceFilterMask::default(),
            active_pill_idx: 0,
            results: Vec::new(),
            selected_result_idx: 0,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.input_query.clear();
        self.results.clear();
        self.selected_result_idx = 0;
    }

    pub fn cycle_filter_pill(&mut self) {
        let sources = [
            SourceKind::LocalFile,
            SourceKind::YouTube,
            SourceKind::Bandcamp,
            SourceKind::SoundCloud,
            SourceKind::Soulseek,
            SourceKind::P2PShared,
        ];
        self.active_pill_idx = (self.active_pill_idx + 1) % sources.len();
        self.filter_mask.toggle(sources[self.active_pill_idx]);
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_result_idx = (self.selected_result_idx + 1) % self.results.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected_result_idx == 0 {
                self.selected_result_idx = self.results.len() - 1;
            } else {
                self.selected_result_idx -= 1;
            }
        }
    }

    pub fn selected_track(&self) -> Option<&Track> {
        self.results.get(self.selected_result_idx)
    }
}

impl Default for SearchViewState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SearchViewModal<'a> {
    pub state: &'a SearchViewState,
}

impl<'a> SearchViewModal<'a> {
    pub fn new(state: &'a SearchViewState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for SearchViewModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.is_open || area.width < 50 || area.height < 10 {
            return;
        }

        // Overlay dialog geometry
        let dialog_width = (area.width - 4).min(76);
        let dialog_height = (area.height - 4).min(20);
        let x0 = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y0 = area.y + (area.height.saturating_sub(dialog_height)) / 2;

        let frame_style = Style::default().bg(Color::Rgb(15, 23, 42)).fg(Color::White);
        let border_style = Style::default()
            .bg(Color::Rgb(15, 23, 42))
            .fg(Color::Rgb(56, 189, 248));
        let input_style = Style::default()
            .bg(Color::Rgb(30, 41, 59))
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let pill_active = Style::default()
            .bg(Color::Rgb(16, 185, 129))
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD);
        let pill_inactive = Style::default()
            .bg(Color::Rgb(51, 65, 85))
            .fg(Color::Rgb(148, 163, 184));
        let selected_row_style = Style::default()
            .bg(Color::Rgb(59, 130, 246))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);

        // Fill background
        for y in y0..(y0 + dialog_height) {
            for x in x0..(x0 + dialog_width) {
                buf[(x, y)].set_style(frame_style);
            }
        }

        // Border top
        let fill_len = (dialog_width as usize).saturating_sub(44);
        let top_title = format!(
            "┌─ UNIVERSAL SEARCH & STREAM AGGREGATOR {}┐",
            "─".repeat(fill_len)
        );
        buf.set_string(x0, y0, &top_title, border_style);

        // Omnibar input line
        let input_line = format!(" Search: {} ", self.state.input_query);
        buf.set_string(x0 + 1, y0 + 1, &input_line, input_style);

        // Filter pills row
        let pills = [
            (SourceKind::LocalFile, "Local"),
            (SourceKind::YouTube, "YouTube"),
            (SourceKind::Bandcamp, "Bandcamp"),
            (SourceKind::SoundCloud, "SoundCloud"),
            (SourceKind::Soulseek, "Soulseek"),
            (SourceKind::P2PShared, "P2P"),
        ];

        let mut pill_x = x0 + 1;
        for (kind, label) in pills {
            let enabled = self.state.filter_mask.is_enabled(kind);
            let check = if enabled { "[✔] " } else { "[ ] " };
            let pill_text = format!(" {check}{label} ");
            let style = if enabled { pill_active } else { pill_inactive };
            buf.set_string(pill_x, y0 + 2, &pill_text, style);
            pill_x += pill_text.len() as u16 + 1;
            if pill_x >= x0 + dialog_width - 1 {
                break;
            }
        }

        // Divider
        let divider = "─".repeat(dialog_width as usize);
        buf.set_string(x0, y0 + 3, &divider, border_style);

        // Results list
        let max_results = (dialog_height as usize).saturating_sub(5);
        for (idx, track) in self.state.results.iter().take(max_results).enumerate() {
            let row_y = y0 + 4 + idx as u16;
            let is_selected = idx == self.state.selected_result_idx;
            let style = if is_selected {
                selected_row_style
            } else {
                frame_style
            };

            let badge = track.source.kind();
            let duration_str = match track.metadata.duration {
                Some(d) => format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60),
                None => "--:--".to_string(),
            };

            let row_text = format!(
                " {} {:<24} - {:<28} [{}] ",
                badge.badge_icon(),
                track.metadata.artist,
                track.metadata.title,
                duration_str
            );
            buf.set_string(x0 + 1, row_y, &row_text, style);
        }

        // Bottom help line
        let bottom_help = " [Enter] Play  [d] Download  [Tab] Toggle Source  [Esc] Close ";
        buf.set_string(x0 + 1, y0 + dialog_height - 1, bottom_help, border_style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};
    use std::time::Duration;

    #[test]
    fn test_search_view_state_and_pill_cycling() {
        let mut state = SearchViewState::new();
        assert!(!state.is_open);
        state.open();
        assert!(state.is_open);

        // Initially all sources enabled
        assert!(state.filter_mask.is_enabled(SourceKind::YouTube));

        // Cycle pill toggles next source
        state.cycle_filter_pill();
        assert!(!state.filter_mask.is_enabled(SourceKind::YouTube));

        state.cycle_filter_pill();
        assert!(!state.filter_mask.is_enabled(SourceKind::Bandcamp));

        // Add mock result and test navigation
        let mut meta = TrackMetadata::new("Windowlicker", "Aphex Twin");
        meta.duration = Some(Duration::from_secs(367));
        state.results.push(Track::new(
            AudioSource::YouTube {
                video_id: "yt_123".into(),
            },
            meta,
        ));

        assert_eq!(state.results.len(), 1);
        assert_eq!(
            state.selected_track().unwrap().metadata.title,
            "Windowlicker"
        );

        // Render test into buffer
        let modal = SearchViewModal::new(&state);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        modal.render(area, &mut buf);

        let mut found_title = false;
        for y in 0..area.height {
            let line: String = (0..area.width)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Windowlicker") {
                found_title = true;
                break;
            }
        }
        assert!(found_title, "Rendered search result missing in buffer");
    }
}
