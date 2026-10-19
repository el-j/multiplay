//! Double size: the whole layout scales around the main window's top-left corner.
//!
//! Window rects are kept in egui points, and a point is `zoom` physical units wide. When the
//! zoom changes, sizes stay the same in points (so each window grows on screen with its
//! content) while positions are re-expressed so the anchor stays put on screen and every
//! offset from it scales. Docked edges stay docked and nothing can start to overlap.

#![forbid(unsafe_code)]

use super::geometry::{DockRect, WindowId};

/// Re-expresses `windows` (in points at `from_zoom`) in points at `to_zoom`, scaling the layout
/// around `anchor`.
pub fn rescale_layout(
    windows: &[(WindowId, DockRect)],
    anchor: WindowId,
    from_zoom: f32,
    to_zoom: f32,
) -> Vec<(WindowId, DockRect)> {
    let Some(a) = windows.iter().find(|(w, _)| *w == anchor).map(|(_, r)| *r) else {
        return windows.to_vec();
    };
    let ratio = from_zoom / to_zoom;
    let (ax, ay) = (a.x * ratio, a.y * ratio);
    windows
        .iter()
        .map(|(id, r)| {
            let moved = DockRect::new(ax + (r.x - a.x), ay + (r.y - a.y), r.width, r.height);
            (*id, moved)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column() -> Vec<(WindowId, DockRect)> {
        vec![
            (WindowId::Main, DockRect::new(120.0, 100.0, 480.0, 210.0)),
            (
                WindowId::Equalizer,
                DockRect::new(120.0, 310.0, 480.0, 200.0),
            ),
            (
                WindowId::Playlist,
                DockRect::new(600.0, 100.0, 300.0, 410.0),
            ),
        ]
    }

    /// Screen position in physical units of a rect expressed in points at `zoom`.
    fn on_screen(r: DockRect, zoom: f32) -> (f32, f32, f32, f32) {
        (r.x * zoom, r.y * zoom, r.width * zoom, r.height * zoom)
    }

    #[test]
    fn main_window_stays_put_on_screen_and_doubles() {
        let out = rescale_layout(&column(), WindowId::Main, 1.0, 2.0);
        let (x, y, w, h) = on_screen(out[0].1, 2.0);
        assert_eq!((x, y, w, h), (120.0, 100.0, 960.0, 420.0));
    }

    #[test]
    fn docked_windows_stay_docked_after_doubling() {
        let out = rescale_layout(&column(), WindowId::Main, 1.0, 2.0);
        let (main, eq, pl) = (out[0].1, out[1].1, out[2].1);
        assert_eq!(eq.top(), main.bottom());
        assert_eq!(eq.left(), main.left());
        assert_eq!(pl.left(), main.right());
        assert_eq!(on_screen(eq, 2.0).1, 100.0 + 420.0);
    }

    #[test]
    fn doubling_twice_back_restores_the_layout() {
        let there = rescale_layout(&column(), WindowId::Main, 1.0, 2.0);
        assert_eq!(rescale_layout(&there, WindowId::Main, 2.0, 1.0), column());
    }
}
