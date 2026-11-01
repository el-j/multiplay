//! Dense Winamp 1.x Faceplate Widget for Ratatui.
//! Renders a compact 78x7 cell retro console faceplate with VFD display,
//! 16-band mini spectrum, volume/balance gauges, buffer fill, and transport controls.

#![forbid(unsafe_code)]

use amp_domain::PlaybackState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::time::Duration;

pub const FACEPLATE_WIDTH: u16 = 78;
pub const FACEPLATE_HEIGHT: u16 = 7;

const SPECTRUM_GLYPHS: [char; 5] = [' ', ' ', '▄', '▆', '█'];

#[derive(Debug, Clone)]
pub struct DenseConsole<'a> {
    pub title: &'a str,
    pub track_num: u32,
    pub elapsed: Duration,
    pub state: PlaybackState,
    pub spectrum: &'a [f32; 16],
    pub volume: u8,  // 0-100
    pub balance: i8, // -50 (L) to +50 (R)
    pub buffer_fill_percent: u8,
    /// Source sample rate; `None` while nothing is decoding, so no format badge is drawn.
    pub sample_rate_khz: Option<f32>,
    /// From the tags of lossy files.
    pub bitrate_kbps: Option<u32>,
    pub is_stereo: bool,
}

impl<'a> DenseConsole<'a> {
    pub fn new(spectrum: &'a [f32; 16]) -> Self {
        Self {
            title: "1. No Track Loaded",
            track_num: 1,
            elapsed: Duration::ZERO,
            state: PlaybackState::Stopped,
            spectrum,
            volume: 80,
            balance: 0,
            buffer_fill_percent: 100,
            sample_rate_khz: None,
            bitrate_kbps: None,
            is_stereo: true,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn track_num(mut self, num: u32) -> Self {
        self.track_num = num;
        self
    }

    pub fn elapsed(mut self, elapsed: Duration) -> Self {
        self.elapsed = elapsed;
        self
    }

    pub fn state(mut self, state: PlaybackState) -> Self {
        self.state = state;
        self
    }

    pub fn volume(mut self, volume: u8) -> Self {
        self.volume = volume.min(100);
        self
    }

    pub fn balance(mut self, balance: i8) -> Self {
        self.balance = balance.clamp(-50, 50);
        self
    }

    pub fn buffer_fill(mut self, percent: u8) -> Self {
        self.buffer_fill_percent = percent.min(100);
        self
    }

    /// The format badge: source rate, bitrate when known, stereo or mono.
    pub fn audio_info(
        mut self,
        sample_rate_khz: f32,
        bitrate_kbps: Option<u32>,
        stereo: bool,
    ) -> Self {
        self.sample_rate_khz = Some(sample_rate_khz);
        self.bitrate_kbps = bitrate_kbps;
        self.is_stereo = stereo;
        self
    }
}

impl<'a> Widget for DenseConsole<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < FACEPLATE_WIDTH || area.height < FACEPLATE_HEIGHT {
            return;
        }

        let bg_style = Style::default()
            .bg(Color::Rgb(24, 24, 27))
            .fg(Color::Rgb(212, 212, 216));
        let title_style = Style::default()
            .bg(Color::Rgb(39, 39, 42))
            .fg(Color::Rgb(250, 204, 21))
            .add_modifier(Modifier::BOLD);
        let vfd_style = Style::default()
            .bg(Color::Rgb(9, 24, 18))
            .fg(Color::Rgb(52, 211, 153))
            .add_modifier(Modifier::BOLD);
        let spec_style = Style::default()
            .bg(Color::Rgb(15, 23, 42))
            .fg(Color::Rgb(56, 189, 248));
        let button_style = Style::default()
            .bg(Color::Rgb(63, 63, 70))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let active_button_style = Style::default()
            .bg(Color::Rgb(34, 197, 94))
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD);
        let badge_style = Style::default()
            .bg(Color::Rgb(24, 24, 27))
            .fg(Color::Rgb(161, 161, 170));

        let x0 = area.x;
        let y0 = area.y;

        // Line 0: Top title bar
        let title_line = format!(" MULTIPLAY :: {:<60} [X]", self.title);
        buf.set_string(x0, y0, &title_line[..FACEPLATE_WIDTH as usize], title_style);

        // Line 1: VFD Time & Format & Mini Spectrum
        let state_indicator = match self.state {
            PlaybackState::Playing => "▶",
            PlaybackState::Paused => "❚❚",
            PlaybackState::Loading | PlaybackState::Buffering { .. } => "⏳",
            PlaybackState::Stopped => "■",
            PlaybackState::Error => "⚠",
        };
        let minutes = self.elapsed.as_secs() / 60;
        let seconds = self.elapsed.as_secs() % 60;
        let vfd_timer = format!(
            " {:02} {} {:02}:{:02} ",
            self.track_num, state_indicator, minutes, seconds
        );
        buf.set_string(x0 + 1, y0 + 1, &vfd_timer, vfd_style);

        // Spectrum bars (16 channels)
        let mut spec_str = String::with_capacity(16);
        for &val in self.spectrum {
            let idx = ((val.clamp(0.0, 1.0) * 4.0).round() as usize).min(4);
            spec_str.push(SPECTRUM_GLYPHS[idx]);
        }
        buf.set_string(x0 + 19, y0 + 1, format!("|{}|", spec_str), spec_style);

        // Audio format badges: kbps, kHz, ST/MO, only for a stream that is really decoding.
        if let Some(khz) = self.sample_rate_khz {
            let kbps = self
                .bitrate_kbps
                .map(|k| format!("[{k}k]"))
                .unwrap_or_default();
            let format_badge = format!(
                "{kbps}[{khz:.1}][{}]",
                if self.is_stereo { "ST" } else { "MO" }
            );
            buf.set_string(x0 + 38, y0 + 1, &format_badge, badge_style);
        }

        // Line 2: Volume & Balance level meters
        let vol_bars = (self.volume as usize * 14) / 100;
        let mut vol_gauge = String::new();
        for i in 0..14 {
            vol_gauge.push(if i < vol_bars { '▮' } else { ' ' });
        }
        let vol_str = format!("VOL [{}%]: [{}]", self.volume, vol_gauge);
        buf.set_string(x0 + 1, y0 + 2, &vol_str, bg_style);

        let bal_str = format!("BAL: [{:<+3}]", self.balance);
        buf.set_string(x0 + 35, y0 + 2, &bal_str, bg_style);

        // Buffer fill indicator
        let buf_str = format!("BUF: {:3}%", self.buffer_fill_percent);
        buf.set_string(x0 + 55, y0 + 2, &buf_str, bg_style);

        // Line 3: Tape-Deck Buttons [PREV] [PLAY] [PAUSE] [STOP] [NEXT] [EJECT]
        let play_style = if self.state == PlaybackState::Playing {
            active_button_style
        } else {
            button_style
        };
        let pause_style = if self.state == PlaybackState::Paused {
            active_button_style
        } else {
            button_style
        };
        let stop_style = if self.state == PlaybackState::Stopped {
            active_button_style
        } else {
            button_style
        };

        buf.set_string(x0 + 2, y0 + 3, "[|<<]", button_style);
        buf.set_string(x0 + 9, y0 + 3, "[ > ]", play_style);
        buf.set_string(x0 + 16, y0 + 3, "[|| ]", pause_style);
        buf.set_string(x0 + 23, y0 + 3, "[ ■ ]", stop_style);
        buf.set_string(x0 + 30, y0 + 3, "[>>|]", button_style);
        buf.set_string(x0 + 37, y0 + 3, "[ ^ ]", button_style);

        // Line 4: Progress position bar
        let progress_width = 74;
        let mut progress_line = String::from("[");
        let fill = (self.elapsed.as_secs() as usize).min(progress_width);
        for i in 0..progress_width {
            progress_line.push(if i < fill { '━' } else { '─' });
        }
        progress_line.push(']');
        buf.set_string(x0 + 1, y0 + 4, &progress_line, bg_style);

        // Line 5: Bottom status and skin text
        let status_line = " EQUALIZER [ON]  PLAYLIST [ON]  VISUALIZER [ON]  MULTI-WINDOW DOCKED ";
        buf.set_string(x0 + 1, y0 + 5, status_line, badge_style);

        // Line 6: Border bottom
        let border_line = "═".repeat(FACEPLATE_WIDTH as usize);
        buf.set_string(x0, y0 + 6, &border_line, bg_style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dense_console_rendering() {
        let spectrum = [
            0.1, 0.3, 0.6, 0.9, 0.8, 0.5, 0.2, 0.0, 0.1, 0.4, 0.7, 0.9, 0.6, 0.3, 0.1, 0.0,
        ];
        let console = DenseConsole::new(&spectrum)
            .title("Aphex Twin - Windowlicker")
            .track_num(3)
            .elapsed(Duration::from_secs(125))
            .state(PlaybackState::Playing)
            .volume(85)
            .balance(-5)
            .buffer_fill(98)
            .audio_info(44.1, Some(320), true);

        let area = Rect::new(0, 0, FACEPLATE_WIDTH, FACEPLATE_HEIGHT);
        let mut buf = Buffer::empty(area);

        console.render(area, &mut buf);

        // Verify VFD string content
        let mut vfd_found = false;
        for y in 0..FACEPLATE_HEIGHT {
            let row: String = (0..FACEPLATE_WIDTH)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if row.contains("03 ▶ 02:05") {
                vfd_found = true;
                break;
            }
        }
        assert!(
            vfd_found,
            "VFD display time string not rendered correctly in buffer"
        );
    }
}
