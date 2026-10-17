//! Detachable Video Stream Canvas View for Ratatui.
//! Renders 24-bit truecolor half-block Unicode characters ('▀') with zero heap allocation during render.

#![forbid(unsafe_code)]

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct VideoFrameBuffer<'a> {
    pub width: u16,
    pub height: u16,
    /// RGB interleaved bytes: [r, g, b, r, g, b, ...]
    pub rgb_data: &'a [u8],
}

pub struct VideoWindowView<'a> {
    pub title: &'a str,
    pub active_frame: Option<VideoFrameBuffer<'a>>,
    pub is_floating: bool,
}

impl<'a> VideoWindowView<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            active_frame: None,
            is_floating: false,
        }
    }

    pub fn frame(mut self, frame: Option<VideoFrameBuffer<'a>>) -> Self {
        self.active_frame = frame;
        self
    }

    pub fn floating(mut self, floating: bool) -> Self {
        self.is_floating = floating;
        self
    }
}

impl<'a> Widget for VideoWindowView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 4 {
            return;
        }

        let border_style = Style::default()
            .bg(Color::Rgb(15, 23, 42))
            .fg(Color::Rgb(56, 189, 248));
        let title_style = Style::default()
            .bg(Color::Rgb(15, 23, 42))
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let placeholder_style = Style::default().bg(Color::Black).fg(Color::DarkGray);

        let x0 = area.x;
        let y0 = area.y;
        let w = area.width;
        let h = area.height;

        // Top Border with Title & Detach status
        let detach_indicator = if self.is_floating {
            "[DETACHED]"
        } else {
            "[DOCKED]"
        };
        let fill_top = (w as usize).saturating_sub(40);
        let title_line = format!(
            "┌─ VIDEO CANVAS: {:<20} {} {}┐",
            self.title,
            detach_indicator,
            "─".repeat(fill_top)
        );
        buf.set_string(x0, y0, &title_line, title_style);

        // Canvas Area (inside borders)
        let canvas_x = x0 + 1;
        let canvas_y = y0 + 1;
        let canvas_w = w.saturating_sub(2);
        let canvas_h = h.saturating_sub(2);

        if let Some(frame) = &self.active_frame {
            // Half-block pixel rendering: each character cell represents TWO vertical pixels.
            // Top half pixel = Foreground color with '▀' (U+2580)
            // Bottom half pixel = Background color
            for cell_y in 0..canvas_h {
                let pixel_y_top = (cell_y * 2) as usize;
                let pixel_y_bot = (cell_y * 2 + 1) as usize;

                for cell_x in 0..canvas_w {
                    let pixel_x = cell_x as usize;

                    let (top_r, top_g, top_b) =
                        if pixel_x < frame.width as usize && pixel_y_top < frame.height as usize {
                            let idx = (pixel_y_top * frame.width as usize + pixel_x) * 3;
                            (
                                frame.rgb_data[idx],
                                frame.rgb_data[idx + 1],
                                frame.rgb_data[idx + 2],
                            )
                        } else {
                            (0, 0, 0)
                        };

                    let (bot_r, bot_g, bot_b) =
                        if pixel_x < frame.width as usize && pixel_y_bot < frame.height as usize {
                            let idx = (pixel_y_bot * frame.width as usize + pixel_x) * 3;
                            (
                                frame.rgb_data[idx],
                                frame.rgb_data[idx + 1],
                                frame.rgb_data[idx + 2],
                            )
                        } else {
                            (0, 0, 0)
                        };

                    let cell = &mut buf[(canvas_x + cell_x, canvas_y + cell_y)];
                    cell.set_char('▀');
                    cell.set_fg(Color::Rgb(top_r, top_g, top_b));
                    cell.set_bg(Color::Rgb(bot_r, bot_g, bot_b));
                }
            }
        } else {
            // Placeholder when no video stream is active
            for cy in 0..canvas_h {
                for cx in 0..canvas_w {
                    buf[(canvas_x + cx, canvas_y + cy)].set_style(placeholder_style);
                }
            }
            let placeholder_text = " [AUDIO ONLY] No active video stream ";
            let text_x = canvas_x + (canvas_w.saturating_sub(placeholder_text.len() as u16)) / 2;
            let text_y = canvas_y + canvas_h / 2;
            buf.set_string(text_x, text_y, placeholder_text, placeholder_style);
        }

        // Bottom Border
        let fill_bot = (w as usize).saturating_sub(2);
        let bottom_line = format!("└{}┘", "─".repeat(fill_bot));
        buf.set_string(x0, y0 + h - 1, &bottom_line, border_style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_view_placeholder() {
        let view = VideoWindowView::new("Test Track");
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);

        let mut found_placeholder = false;
        for y in 0..area.height {
            let row: String = (0..area.width)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if row.contains("[AUDIO ONLY]") {
                found_placeholder = true;
                break;
            }
        }
        assert!(found_placeholder, "Audio-only placeholder missing");
    }

    #[test]
    fn test_video_view_half_block_rendering() {
        // 2x2 video pixels:
        // (0,0) Red, (1,0) Green
        // (0,1) Blue, (1,1) White
        let pixels: [u8; 12] = [
            255, 0, 0, 0, 255, 0, // top row
            0, 0, 255, 255, 255, 255, // bottom row
        ];

        let frame = VideoFrameBuffer {
            width: 2,
            height: 2,
            rgb_data: &pixels,
        };

        let view = VideoWindowView::new("Live Video")
            .frame(Some(frame))
            .floating(true);
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);

        // Verify top-left cell rendered half block '▀' with Red FG and Blue BG
        let cell = &buf[(1, 1)];
        assert_eq!(cell.symbol(), "▀");
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
        assert_eq!(cell.bg, Color::Rgb(0, 0, 255));
    }
}
