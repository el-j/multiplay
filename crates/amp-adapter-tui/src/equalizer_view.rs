//! Equalizer widget for the classic Winamp 10-band interface.
//! Renders preamp and 10 frequency band gain sliders in terminal character cells.

#![forbid(unsafe_code)]

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct EqualizerWidget<'a> {
    gains: &'a [f32; 10],
    is_enabled: bool,
    preamp_db: f32,
}

impl<'a> EqualizerWidget<'a> {
    pub fn new(gains: &'a [f32; 10], is_enabled: bool, preamp_db: f32) -> Self {
        Self {
            gains,
            is_enabled,
            preamp_db,
        }
    }
}

impl<'a> Widget for EqualizerWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 30 || area.height < 4 {
            return;
        }

        let bg_style = Style::default()
            .bg(Color::Rgb(16, 20, 24))
            .fg(Color::DarkGray);
        let title_style = Style::default()
            .bg(Color::Rgb(24, 32, 40))
            .fg(Color::Rgb(0, 235, 136))
            .add_modifier(Modifier::BOLD);
        let on_style = if self.is_enabled {
            Style::default().bg(Color::Rgb(0, 160, 90)).fg(Color::White)
        } else {
            Style::default()
                .bg(Color::Rgb(40, 40, 40))
                .fg(Color::DarkGray)
        };
        let slider_style = Style::default().fg(Color::Rgb(50, 180, 255));
        let zero_style = Style::default().fg(Color::DarkGray);

        // Fill background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_style(bg_style);
            }
        }

        // Line 0: Header
        let header = " ═══ WINAMP EQUALIZER ══════════════════════════════════════════════════════";
        buf.set_string(area.left(), area.top(), header, bg_style);
        buf.set_string(area.left() + 5, area.top(), "WINAMP EQUALIZER", title_style);

        // Line 1: Toggles & Preamp
        let toggle_txt = if self.is_enabled { " [ON] " } else { " [OFF] " };
        buf.set_string(area.left() + 2, area.top() + 1, toggle_txt, on_style);
        buf.set_string(area.left() + 9, area.top() + 1, " [AUTO] ", bg_style);
        buf.set_string(
            area.left() + 18,
            area.top() + 1,
            format!("PREAMP [{:+04.1}dB]", self.preamp_db),
            slider_style,
        );

        // Line 2 & 3: 10 Bands (60, 170, 310, 600, 1k, 3k, 6k, 12k, 14k, 16k)
        let labels = [
            "60", "170", "310", "600", "1k", "3k", "6k", "12k", "14k", "16k",
        ];
        let col_width = (area.width.saturating_sub(4) / 10).max(5);

        for (i, (&gain, label)) in self.gains.iter().zip(labels.iter()).enumerate() {
            let x = area.left() + 2 + (i as u16 * col_width);
            if x + 4 >= area.right() {
                break;
            }

            // Slider representation
            let symbol = if gain > 4.0 {
                " ▲▲ "
            } else if gain > 0.0 {
                "  ▲ "
            } else if gain == 0.0 {
                " ── "
            } else if gain > -4.0 {
                "  ▼ "
            } else {
                " ▼▼ "
            };

            let s_style = if gain == 0.0 {
                zero_style
            } else {
                slider_style
            };
            buf.set_string(x, area.top() + 2, symbol, s_style);
            buf.set_string(x, area.top() + 3, format!("{label:^4}"), bg_style);
        }

        // Bottom border
        if area.height >= 5 {
            let border = "═".repeat(area.width as usize);
            buf.set_string(area.left(), area.top() + 4, border, bg_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equalizer_widget_rendering() {
        let gains = [0.0; 10];
        let widget = EqualizerWidget::new(&gains, true, 0.0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 78, 6));
        widget.render(Rect::new(0, 0, 78, 6), &mut buf);

        let content = buf.content();
        assert!(!content.is_empty());
    }
}
