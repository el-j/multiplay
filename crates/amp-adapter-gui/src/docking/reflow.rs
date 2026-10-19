//! Keeping windows apart: resizing with docked neighbours following the moving edge, and
//! pushing windows out of each other so that no two visible windows ever overlap.

#![forbid(unsafe_code)]

use super::geometry::{DockRect, WindowId};
use super::snap::TOUCH_EPSILON;

/// True when two rects share interior area (touching edges do not count).
pub fn overlaps(a: &DockRect, b: &DockRect) -> bool {
    a.left() < b.right() - TOUCH_EPSILON
        && a.right() > b.left() + TOUCH_EPSILON
        && a.top() < b.bottom() - TOUCH_EPSILON
        && a.bottom() > b.top() + TOUCH_EPSILON
}

/// Moves `moving` the shortest distance that takes it out of every rect in `others`, so it ends
/// flush against them instead of on top of them.
pub fn separate_from(moving: DockRect, others: &[DockRect]) -> DockRect {
    let mut r = moving;
    for _ in 0..8 {
        let Some(o) = others.iter().find(|o| overlaps(&r, o)) else {
            break;
        };
        let pushes = [
            (o.left() - r.right(), 0.0),
            (o.right() - r.left(), 0.0),
            (0.0, o.top() - r.bottom()),
            (0.0, o.bottom() - r.top()),
        ];
        let (dx, dy) = pushes
            .into_iter()
            .min_by(|a, b| (a.0.abs() + a.1.abs()).total_cmp(&(b.0.abs() + b.1.abs())))
            .unwrap_or((0.0, 0.0));
        r.translate(dx, dy);
    }
    r
}

/// Edge of a window that is being moved by a size change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Right,
    Bottom,
}

/// Windows docked beyond `side` of `id`: touching that edge, plus windows that touch those and
/// lie entirely beyond the edge (a docked chain).
pub fn attached_beyond(
    id: WindowId,
    side: Side,
    windows: &[(WindowId, DockRect)],
) -> Vec<WindowId> {
    let Some(base) = windows.iter().find(|(w, _)| *w == id).map(|(_, r)| *r) else {
        return Vec::new();
    };
    let beyond = |r: &DockRect| match side {
        Side::Right => r.left() >= base.right() - TOUCH_EPSILON,
        Side::Bottom => r.top() >= base.bottom() - TOUCH_EPSILON,
    };
    let near = |a: f32, b: f32| (a - b).abs() <= TOUCH_EPSILON;
    let on_edge = |r: &DockRect| match side {
        Side::Right => near(r.left(), base.right()) && r.overlaps_vertical(&base),
        Side::Bottom => near(r.top(), base.bottom()) && r.overlaps_horizontal(&base),
    };
    let mut out: Vec<WindowId> = windows
        .iter()
        .filter(|(w, r)| *w != id && on_edge(r))
        .map(|(w, _)| *w)
        .collect();
    let mut i = 0;
    while i < out.len() {
        let cur = windows.iter().find(|(w, _)| *w == out[i]).map(|(_, r)| *r);
        if let Some(cur) = cur {
            for (w, r) in windows {
                if *w != id && !out.contains(w) && beyond(r) && super::snap::touching(&cur, r) {
                    out.push(*w);
                }
            }
        }
        i += 1;
    }
    out
}

/// Limits a requested size so the window does not grow into windows that are not docked to it.
pub fn clamp_growth(
    id: WindowId,
    want_w: f32,
    want_h: f32,
    windows: &[(WindowId, DockRect)],
) -> (f32, f32) {
    let Some(base) = windows.iter().find(|(w, _)| *w == id).map(|(_, r)| *r) else {
        return (want_w, want_h);
    };
    let right = attached_beyond(id, Side::Right, windows);
    let below = attached_beyond(id, Side::Bottom, windows);
    let (mut w, mut h) = (want_w, want_h);
    for (other, r) in windows {
        if *other == id || right.contains(other) || below.contains(other) {
            continue;
        }
        let grown = DockRect::new(base.x, base.y, w, h);
        if !overlaps(&grown, r) {
            continue;
        }
        if r.left() >= base.right() - TOUCH_EPSILON {
            w = w.min(r.left() - base.x);
        }
        if r.top() >= base.bottom() - TOUCH_EPSILON {
            h = h.min(r.top() - base.y);
        }
    }
    (
        w.max(base.width.min(want_w)),
        h.max(base.height.min(want_h)),
    )
}

/// Resizes `id` to `w` x `h`. Windows docked beyond its right or bottom edge move with that
/// edge; any other window left overlapping is pushed out. Returns every changed rect.
pub fn resize_with_attached(
    id: WindowId,
    w: f32,
    h: f32,
    windows: &[(WindowId, DockRect)],
) -> Vec<(WindowId, DockRect)> {
    let Some(base) = windows.iter().find(|(win, _)| *win == id).map(|(_, r)| *r) else {
        return Vec::new();
    };
    let (dw, dh) = (w - base.width, h - base.height);
    let right = attached_beyond(id, Side::Right, windows);
    let below = attached_beyond(id, Side::Bottom, windows);
    let mut out: Vec<(WindowId, DockRect)> = windows.to_vec();
    for (win, r) in out.iter_mut() {
        if *win == id {
            *r = DockRect::new(base.x, base.y, w, h);
        } else {
            let dx = if right.contains(win) { dw } else { 0.0 };
            let dy = if below.contains(win) { dh } else { 0.0 };
            r.translate(dx, dy);
        }
    }
    push_apart(id, &mut out);
    out.into_iter()
        .filter(|(win, r)| windows.iter().any(|(w0, r0)| w0 == win && r0 != r))
        .collect()
}

/// Pushes windows out of each other, never moving `anchor`.
pub fn push_apart(anchor: WindowId, windows: &mut [(WindowId, DockRect)]) {
    for _ in 0..16 {
        let mut moved = false;
        for i in 0..windows.len() {
            if windows[i].0 == anchor {
                continue;
            }
            let others: Vec<DockRect> = windows
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, (_, r))| *r)
                .collect();
            let freed = separate_from(windows[i].1, &others);
            if freed != windows[i].1 {
                windows[i].1 = freed;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> DockRect {
        DockRect::new(x, y, w, h)
    }

    fn get(v: &[(WindowId, DockRect)], id: WindowId) -> Option<DockRect> {
        v.iter().find(|(w, _)| *w == id).map(|(_, r)| *r)
    }

    #[test]
    fn dropped_window_ends_flush_instead_of_on_top() {
        let main = r(100.0, 100.0, 480.0, 210.0);
        let moved = separate_from(r(120.0, 290.0, 480.0, 200.0), &[main]);
        assert_eq!(moved.y, 310.0);
        assert!(!overlaps(&moved, &main));
    }

    #[test]
    fn growing_height_moves_the_docked_column_below() {
        let w = vec![
            (WindowId::Main, r(0.0, 0.0, 480.0, 210.0)),
            (WindowId::Equalizer, r(0.0, 210.0, 480.0, 200.0)),
            (WindowId::Playlist, r(0.0, 410.0, 480.0, 300.0)),
        ];
        let out = resize_with_attached(WindowId::Main, 480.0, 260.0, &w);
        assert_eq!(get(&out, WindowId::Equalizer).unwrap().y, 260.0);
        assert_eq!(get(&out, WindowId::Playlist).unwrap().y, 460.0);
    }

    #[test]
    fn shading_pulls_the_docked_column_up() {
        let w = vec![
            (WindowId::Main, r(0.0, 0.0, 480.0, 210.0)),
            (WindowId::Equalizer, r(0.0, 210.0, 480.0, 200.0)),
        ];
        let out = resize_with_attached(WindowId::Main, 480.0, 14.0, &w);
        assert_eq!(get(&out, WindowId::Equalizer).unwrap().y, 14.0);
    }

    #[test]
    fn growing_width_moves_the_window_docked_on_the_right_only() {
        let w = vec![
            (WindowId::Main, r(0.0, 0.0, 480.0, 210.0)),
            (WindowId::Playlist, r(480.0, 0.0, 300.0, 300.0)),
            (WindowId::Equalizer, r(0.0, 210.0, 480.0, 200.0)),
        ];
        let out = resize_with_attached(WindowId::Main, 520.0, 210.0, &w);
        assert_eq!(get(&out, WindowId::Playlist).unwrap().x, 520.0);
        assert!(
            get(&out, WindowId::Equalizer).is_none(),
            "EQ below must not move"
        );
    }

    #[test]
    fn growth_stops_at_a_window_that_is_not_docked() {
        let w = vec![
            (WindowId::Playlist, r(0.0, 0.0, 300.0, 300.0)),
            (WindowId::Equalizer, r(350.0, 0.0, 300.0, 200.0)),
        ];
        let (cw, ch) = clamp_growth(WindowId::Playlist, 500.0, 300.0, &w);
        assert_eq!((cw, ch), (350.0, 300.0));
    }

    #[test]
    fn no_two_windows_overlap_after_a_resize_sequence() {
        let mut w = vec![
            (WindowId::Main, r(0.0, 0.0, 480.0, 210.0)),
            (WindowId::Equalizer, r(0.0, 210.0, 480.0, 200.0)),
            (WindowId::Playlist, r(480.0, 0.0, 300.0, 410.0)),
            (WindowId::DjConsole, r(200.0, 420.0, 360.0, 200.0)),
        ];
        for (id, nw, nh) in [
            (WindowId::Main, 600.0, 260.0),
            (WindowId::Equalizer, 300.0, 400.0),
            (WindowId::Main, 480.0, 14.0),
            (WindowId::Main, 480.0, 300.0),
            (WindowId::Playlist, 500.0, 700.0),
        ] {
            let (cw, ch) = clamp_growth(id, nw, nh, &w);
            for (win, rect) in resize_with_attached(id, cw, ch, &w) {
                if let Some(slot) = w.iter_mut().find(|(x, _)| *x == win) {
                    slot.1 = rect;
                }
            }
            for i in 0..w.len() {
                for j in i + 1..w.len() {
                    assert!(
                        !overlaps(&w[i].1, &w[j].1),
                        "{:?} overlaps {:?}",
                        w[i],
                        w[j]
                    );
                }
            }
        }
    }
}
