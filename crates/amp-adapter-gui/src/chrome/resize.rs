//! Window size policy and app-driven resizing.

#![forbid(unsafe_code)]

use crate::docking::{DockRect, WindowId};
use eframe::egui::Vec2;

/// Smallest inner size (including the title strip) at which a window's content still fits.
pub fn min_inner_size(window: WindowId) -> Vec2 {
    match window {
        WindowId::Main => Vec2::new(300.0, 200.0),
        WindowId::Equalizer => Vec2::new(300.0, 170.0),
        WindowId::Playlist => Vec2::new(300.0, 180.0),
        WindowId::Settings => Vec2::new(360.0, 260.0),
        WindowId::DjConsole => Vec2::new(360.0, 200.0),
        WindowId::Remote => Vec2::new(300.0, 200.0),
        WindowId::Search => Vec2::new(360.0, 260.0),
    }
}

/// One resize gesture on a window's right edge, bottom edge or bottom-right grip.
#[derive(Debug, Clone, PartialEq)]
pub struct ResizeSession {
    pub window: WindowId,
    pub right: bool,
    pub bottom: bool,
    press: (f32, f32),
    start: DockRect,
    min: Vec2,
}

impl ResizeSession {
    pub fn start(
        window: WindowId,
        right: bool,
        bottom: bool,
        press: (f32, f32),
        start: DockRect,
    ) -> Self {
        Self {
            window,
            right,
            bottom,
            press,
            start,
            min: min_inner_size(window),
        }
    }

    /// Uses `min` as the smallest size instead of the window's default minimum.
    pub fn with_min(mut self, min: Vec2) -> Self {
        self.min = min;
        self
    }

    /// Size the window should take for the pointer at `pointer` (screen points): follows the
    /// pointer on the dragged edges, never below the window's minimum, with the moving edges
    /// snapping to other windows' and the screen's edges.
    pub fn target_size(
        &self,
        pointer: (f32, f32),
        others: &[DockRect],
        screens: &[DockRect],
        distance: f32,
    ) -> (f32, f32) {
        let min = self.min;
        let mut w = self.start.width;
        let mut h = self.start.height;
        if self.right {
            let edge = self.start.right() + pointer.0 - self.press.0;
            let xs = others
                .iter()
                .flat_map(|o| [o.left(), o.right()])
                .chain(screens.iter().map(|s| s.right()));
            w = snap_edge(edge, xs, distance) - self.start.x;
        }
        if self.bottom {
            let edge = self.start.bottom() + pointer.1 - self.press.1;
            let ys = others
                .iter()
                .flat_map(|o| [o.top(), o.bottom()])
                .chain(screens.iter().map(|s| s.bottom()));
            h = snap_edge(edge, ys, distance) - self.start.y;
        }
        (w.max(min.x), h.max(min.y))
    }
}

fn snap_edge(edge: f32, candidates: impl Iterator<Item = f32>, distance: f32) -> f32 {
    candidates
        .filter(|c| (c - edge).abs() <= distance)
        .min_by(|a, b| (a - edge).abs().total_cmp(&(b - edge).abs()))
        .unwrap_or(edge)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(right: bool, bottom: bool) -> ResizeSession {
        let start = DockRect::new(100.0, 100.0, 480.0, 300.0);
        ResizeSession::start(WindowId::Playlist, right, bottom, (580.0, 400.0), start)
    }

    #[test]
    fn grip_follows_the_pointer_on_both_axes() {
        let s = session(true, true);
        assert_eq!(
            s.target_size((630.0, 450.0), &[], &[], 10.0),
            (530.0, 350.0)
        );
    }

    #[test]
    fn edge_resize_changes_only_its_axis() {
        assert_eq!(
            session(true, false).target_size((630.0, 999.0), &[], &[], 10.0),
            (530.0, 300.0)
        );
        assert_eq!(
            session(false, true).target_size((999.0, 450.0), &[], &[], 10.0),
            (480.0, 350.0)
        );
    }

    #[test]
    fn never_smaller_than_the_window_minimum() {
        let min = min_inner_size(WindowId::Playlist);
        assert_eq!(
            session(true, true).target_size((0.0, 0.0), &[], &[], 10.0),
            (min.x, min.y)
        );
    }

    #[test]
    fn moving_edge_snaps_to_nearby_window_edges() {
        let neighbour = DockRect::new(700.0, 100.0, 200.0, 200.0);
        let s = session(true, false);
        // Right edge dragged to 694: within 10 pt of the neighbour's left edge (700).
        assert_eq!(
            s.target_size((694.0, 400.0), &[neighbour], &[], 10.0).0,
            600.0
        );
    }
}
