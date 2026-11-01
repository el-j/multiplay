//! What the TUI draws on top of its windows (TASK-174): the `?` help, the add prompt, the quit
//! question and a one-line status.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayApp;
use crate::tui_keys::KEY_HELP;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// A `width` x `height` box centred in `area`, never larger than it.
fn centred(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

fn boxed(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Rgb(0, 235, 136)))
}

impl MultiplayApp {
    /// Draws the overlays that are open, after the windows.
    pub(crate) fn render_overlays(&self, frame: &mut Frame) {
        let area = frame.area();
        if let Some(status) = &self.status {
            let line = Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            );
            frame.render_widget(
                Paragraph::new(status.as_str()).style(Style::default().fg(Color::Rgb(0, 200, 240))),
                line,
            );
        }
        if self.show_help {
            let key = Style::default()
                .fg(Color::Rgb(255, 200, 80))
                .add_modifier(Modifier::BOLD);
            let lines: Vec<Line> = KEY_HELP
                .iter()
                .map(|(k, what)| {
                    Line::from(vec![
                        Span::styled(format!("{k:>10}  "), key),
                        Span::raw(*what),
                    ])
                })
                .collect();
            let rect = centred(area, 52, KEY_HELP.len() as u16 + 2);
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(lines).block(boxed(" Keys (any key closes) ")),
                rect,
            );
        }
        if let Some(prompt) = &self.prompt {
            let rect = centred(area, 72, 4);
            frame.render_widget(Clear, rect);
            let text = vec![
                Line::from(format!("{}▏", prompt.text)),
                Line::from(Span::styled(
                    "Tab completes · Enter adds · Esc cancels",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            frame.render_widget(
                Paragraph::new(text).block(boxed(" Add file, folder or link ")),
                rect,
            );
        }
        if self.confirm_quit {
            let rect = centred(area, 34, 3);
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new("Quit while playing? (y / n)").block(boxed(" multiplay ")),
                rect,
            );
        }
    }
}
