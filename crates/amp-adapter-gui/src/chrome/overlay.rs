//! The settings overlay: a one-time window that opens centered, never docks or snaps, and
//! closes with its close button, Esc, Done, or the same shortcut that opened it.

#![forbid(unsafe_code)]

use crate::docking::DockRect;
use eframe::egui::{Key, KeyboardShortcut, Modifiers};

/// Ctrl+P (Cmd+P on macOS) opens and closes Settings, as in classic Winamp.
pub const SETTINGS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::P);

/// Where a `w` x `h` overlay opens: centered on the screen when it is known, otherwise over the
/// main window; never above or left of the screen's top-left corner.
pub fn centered(w: f32, h: f32, screen: Option<DockRect>, main: DockRect) -> DockRect {
    let area = screen.unwrap_or(main);
    let mut x = area.x + (area.width - w) / 2.0;
    let mut y = area.y + (area.height - h) / 2.0;
    if let Some(s) = screen {
        x = x.max(s.x);
        y = y.max(s.y);
    }
    DockRect::new(x.round(), y.round(), w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: DockRect = DockRect {
        x: 120.0,
        y: 100.0,
        width: 480.0,
        height: 210.0,
    };

    #[test]
    fn settings_opens_centered_on_the_screen() {
        let screen = DockRect::new(0.0, 0.0, 1440.0, 900.0);
        let r = centered(520.0, 420.0, Some(screen), MAIN);
        assert_eq!((r.x, r.y), (460.0, 240.0));
    }

    #[test]
    fn settings_centers_over_main_without_screen_info() {
        let r = centered(520.0, 420.0, None, MAIN);
        assert_eq!((r.x, r.y), (100.0, -5.0));
    }

    #[test]
    fn settings_larger_than_the_screen_starts_at_its_corner() {
        let screen = DockRect::new(0.0, 0.0, 400.0, 300.0);
        let r = centered(520.0, 420.0, Some(screen), MAIN);
        assert_eq!((r.x, r.y), (0.0, 0.0));
    }
}
