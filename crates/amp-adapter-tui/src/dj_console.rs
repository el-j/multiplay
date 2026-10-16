//! Hardware-style DJ Console widget for ratatui TUI.
//! Displays dual decks (Deck A & Deck B), central crossfader mixer, beat grids, and isolator kill EQs.

#![forbid(unsafe_code)]

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// UI view model for a single DJ deck.
#[derive(Debug, Clone)]
pub struct DeckViewModel {
    pub title: String,
    pub artist: String,
    pub bpm: f32,
    pub key: String,
    pub pitch_percent: f32,
    pub progress_percent: f32,
    pub volume_fader: f32,
    pub kill_low: bool,
    pub kill_mid: bool,
    pub kill_high: bool,
    pub cue_pfl: bool,
}

impl Default for DeckViewModel {
    fn default() -> Self {
        Self {
            title: "No Track Loaded".into(),
            artist: "-".into(),
            bpm: 120.0,
            key: "8A".into(),
            pitch_percent: 0.0,
            progress_percent: 0.0,
            volume_fader: 0.8,
            kill_low: false,
            kill_mid: false,
            kill_high: false,
            cue_pfl: false,
        }
    }
}

/// Dual-deck DJ console widget with central mixer and crossfader.
pub struct DjConsoleWidget<'a> {
    pub deck_a: &'a DeckViewModel,
    pub deck_b: &'a DeckViewModel,
    pub crossfader_pos: f32, // -1.0 to 1.0
    pub is_sharp_cut: bool,
    pub key_lock_enabled: bool,
}

impl<'a> DjConsoleWidget<'a> {
    #[must_use]
    pub fn new(
        deck_a: &'a DeckViewModel,
        deck_b: &'a DeckViewModel,
        crossfader_pos: f32,
        is_sharp_cut: bool,
        key_lock_enabled: bool,
    ) -> Self {
        Self {
            deck_a,
            deck_b,
            crossfader_pos,
            is_sharp_cut,
            key_lock_enabled,
        }
    }
}

impl<'a> Widget for DjConsoleWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 50 || area.height < 8 {
            return;
        }

        let bg_style = Style::default().bg(Color::Rgb(12, 16, 22)).fg(Color::DarkGray);
        let header_style = Style::default()
            .bg(Color::Rgb(20, 28, 38))
            .fg(Color::Rgb(0, 235, 136))
            .add_modifier(Modifier::BOLD);
        let deck_a_style = Style::default().fg(Color::Rgb(60, 180, 255)).add_modifier(Modifier::BOLD);
        let deck_b_style = Style::default().fg(Color::Rgb(255, 140, 40)).add_modifier(Modifier::BOLD);
        let text_style = Style::default().fg(Color::White);
        let active_cue_style = Style::default().bg(Color::Rgb(230, 40, 40)).fg(Color::White).add_modifier(Modifier::BOLD);
        let idle_cue_style = Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::DarkGray);

        // Fill background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_style(bg_style);
            }
        }

        // Header bar
        let header_title = " ═══ MULTIPLAY PRO DJ CONSOLE ══════════════════════════════════════════════";
        buf.set_string(area.left(), area.top(), header_title, bg_style);
        buf.set_string(area.left() + 5, area.top(), "MULTIPLAY PRO DJ CONSOLE", header_style);

        let half_w = area.width / 2;
        let y_start = area.top() + 2;

        // --- DECK A (Left) ---
        buf.set_string(area.left() + 2, y_start, "[ DECK A ]", deck_a_style);
        let info_a = format!(
            "BPM: {:.1}  Key: {}  Pitch: {:+.1}%",
            self.deck_a.bpm, self.deck_a.key, self.deck_a.pitch_percent
        );
        buf.set_string(area.left() + 14, y_start, &info_a, text_style);
        let track_a = format!("♪ {} - {}", self.deck_a.artist, self.deck_a.title);
        buf.set_string(area.left() + 2, y_start + 1, &track_a[..track_a.len().min(half_w as usize - 4)], text_style);

        // Deck A Kill EQs
        let eq_a = format!(
            "EQ: [HI: {}] [MID: {}] [LOW: {}]",
            if self.deck_a.kill_high { "KILL" } else { "ON" },
            if self.deck_a.kill_mid { "KILL" } else { "ON" },
            if self.deck_a.kill_low { "KILL" } else { "ON" }
        );
        buf.set_string(area.left() + 2, y_start + 2, &eq_a, text_style);

        // Deck A Cue PFL
        let cue_a_style = if self.deck_a.cue_pfl { active_cue_style } else { idle_cue_style };
        buf.set_string(area.left() + 2, y_start + 3, "[CUE A]", cue_a_style);

        // --- DECK B (Right) ---
        let right_x = area.left() + half_w + 1;
        buf.set_string(right_x, y_start, "[ DECK B ]", deck_b_style);
        let info_b = format!(
            "BPM: {:.1}  Key: {}  Pitch: {:+.1}%",
            self.deck_b.bpm, self.deck_b.key, self.deck_b.pitch_percent
        );
        buf.set_string(right_x + 12, y_start, &info_b, text_style);
        let track_b = format!("♪ {} - {}", self.deck_b.artist, self.deck_b.title);
        buf.set_string(right_x, y_start + 1, &track_b[..track_b.len().min(half_w as usize - 4)], text_style);

        // Deck B Kill EQs
        let eq_b = format!(
            "EQ: [HI: {}] [MID: {}] [LOW: {}]",
            if self.deck_b.kill_high { "KILL" } else { "ON" },
            if self.deck_b.kill_mid { "KILL" } else { "ON" },
            if self.deck_b.kill_low { "KILL" } else { "ON" }
        );
        buf.set_string(right_x, y_start + 2, &eq_b, text_style);

        // Deck B Cue PFL
        let cue_b_style = if self.deck_b.cue_pfl { active_cue_style } else { idle_cue_style };
        buf.set_string(right_x, y_start + 3, "[CUE B]", cue_b_style);

        // --- Central Crossfader ---
        let cross_y = y_start + 4;
        let fader_width = 20.min(area.width.saturating_sub(10) as usize);
        let center_x = area.left() + (area.width.saturating_sub(fader_width as u16)) / 2;
        let norm_fader = ((self.crossfader_pos + 1.0) * 0.5 * (fader_width as f32)).round() as usize;

        let mut fader_track = String::with_capacity(fader_width);
        for i in 0..fader_width {
            if i == norm_fader.min(fader_width - 1) {
                fader_track.push('█');
            } else if i == fader_width / 2 {
                fader_track.push('|');
            } else {
                fader_track.push('─');
            }
        }

        let curve_label = if self.is_sharp_cut { "SHARP CUT" } else { "SMOOTH" };
        let fader_line = format!("[A] {} [B] ({})", fader_track, curve_label);
        buf.set_string(center_x.saturating_sub(4), cross_y, &fader_line, text_style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dj_console_rendering() {
        let deck_a = DeckViewModel {
            title: "Techno Track".into(),
            artist: "DJ 1".into(),
            bpm: 128.0,
            key: "11B".into(),
            pitch_percent: 1.5,
            progress_percent: 0.35,
            volume_fader: 0.9,
            kill_low: false,
            kill_mid: false,
            kill_high: false,
            cue_pfl: true,
        };

        let deck_b = DeckViewModel {
            title: "House Anthem".into(),
            artist: "DJ 2".into(),
            bpm: 124.0,
            key: "11B".into(),
            pitch_percent: -0.5,
            progress_percent: 0.10,
            volume_fader: 0.7,
            kill_low: true,
            kill_mid: false,
            kill_high: false,
            cue_pfl: false,
        };

        let widget = DjConsoleWidget::new(&deck_a, &deck_b, 0.0, false, true);
        let area = Rect::new(0, 0, 100, 12);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);

        // Check buffer content contains deck labels
        let mut found_deck_a = false;
        let mut found_deck_b = false;
        for y in 0..12 {
            let mut line = String::new();
            for x in 0..100 {
                line.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            if line.contains("[ DECK A ]") {
                found_deck_a = true;
            }
            if line.contains("[ DECK B ]") {
                found_deck_b = true;
            }
        }

        assert!(found_deck_a, "Deck A label must be rendered");
        assert!(found_deck_b, "Deck B label must be rendered");
    }
}
