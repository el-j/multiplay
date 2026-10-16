//! Playlist editor widget for classic Winamp pledit interface.
//! Renders tracklist items, selection indicator, and bottom transport statistics.

#![forbid(unsafe_code)]

use amp_domain::Track;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct PlaylistWidget<'a> {
    tracks: &'a [Track],
    selected_index: usize,
    total_duration_secs: u64,
}

impl<'a> PlaylistWidget<'a> {
    pub fn new(tracks: &'a [Track], selected_index: usize, total_duration_secs: u64) -> Self {
        Self {
            tracks,
            selected_index,
            total_duration_secs,
        }
    }
}

impl<'a> Widget for PlaylistWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 30 || area.height < 4 {
            return;
        }

        let bg_style = Style::default().bg(Color::Rgb(12, 16, 20)).fg(Color::DarkGray);
        let title_style = Style::default()
            .bg(Color::Rgb(20, 28, 36))
            .fg(Color::Rgb(0, 235, 136))
            .add_modifier(Modifier::BOLD);
        let row_style = Style::default().fg(Color::Rgb(180, 210, 230));
        let selected_style = Style::default()
            .bg(Color::Rgb(0, 80, 140))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let badge_style = Style::default().fg(Color::Rgb(255, 200, 50));

        // Fill background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_style(bg_style);
            }
        }

        // Header
        let header = " ═══ WINAMP PLAYLIST ═══════════════════════════════════════════════════════";
        buf.set_string(area.left(), area.top(), header, bg_style);
        buf.set_string(area.left() + 5, area.top(), "WINAMP PLAYLIST", title_style);

        // Visible track rows
        let available_rows = area.height.saturating_sub(2) as usize;
        let start_idx = if self.selected_index >= available_rows {
            self.selected_index - available_rows + 1
        } else {
            0
        };

        for (row_offset, track_idx) in (start_idx..self.tracks.len()).take(available_rows).enumerate() {
            let y = area.top() + 1 + row_offset as u16;
            let track = &self.tracks[track_idx];
            let is_sel = track_idx == self.selected_index;
            let line_style = if is_sel { selected_style } else { row_style };

            let cursor_sym = if is_sel { "▶ " } else { "  " };
            let title = format!(
                "{}{}. {} - {}",
                cursor_sym,
                track_idx + 1,
                track.metadata.artist,
                track.metadata.title
            );

            let max_w = area.width.saturating_sub(12) as usize;
            let display_title = if title.len() > max_w {
                format!("{}...", &title[..max_w.saturating_sub(3)])
            } else {
                format!("{title:<max_w$}")
            };

            buf.set_string(area.left() + 1, y, &display_title, line_style);

            // Duration
            let dur_str = if let Some(dur) = track.metadata.duration {
                let s = dur.as_secs();
                format!("{:02}:{:02}", s / 60, s % 60)
            } else {
                "--:--".to_string()
            };
            buf.set_string(area.right().saturating_sub(7), y, dur_str, line_style);
        }

        // Bottom status bar
        if area.height >= 3 {
            let mins = self.total_duration_secs / 60;
            let secs = self.total_duration_secs % 60;
            let status = format!(
                " {} TRACKS | {:02}:{:02} TOTAL | [+ FILE] [- REMOVE] [MISC OPTS] ",
                self.tracks.len(),
                mins,
                secs
            );
            buf.set_string(
                area.left() + 1,
                area.bottom().saturating_sub(1),
                status,
                badge_style,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};
    use std::time::Duration;

    #[test]
    fn test_playlist_widget_rendering() {
        let mut meta = TrackMetadata::new("Windowlicker", "Aphex Twin");
        meta.duration = Some(Duration::from_secs(367));
        let track = Track::new(
            AudioSource::DirectHttp {
                url: "http://example.com/audio.mp3".to_string(),
            },
            meta,
        );

        let tracks = vec![track];
        let widget = PlaylistWidget::new(&tracks, 0, 367);
        let mut buf = Buffer::empty(Rect::new(0, 0, 78, 10));
        widget.render(Rect::new(0, 0, 78, 10), &mut buf);

        let content = buf.content();
        assert!(!content.is_empty());
    }
}
