//! Live magnetic snapping: pure geometry used while a window is being dragged.
//! Windows snap flush to other windows' edges and to screen work-area edges, and line up
//! their side edges when they touch, like the classic Winamp 2 window set.

#![forbid(unsafe_code)]

use super::geometry::{DockRect, WindowId};

/// Edges closer than this are considered touching (docked).
pub const TOUCH_EPSILON: f32 = 0.5;

/// Returns `moving` moved so that its edges snap to nearby window or screen edges.
///
/// Each axis snaps independently to the closest candidate within `distance`.
/// Window edges only attract when the rects are near each other on the other axis.
pub fn snap_rect(
    moving: DockRect,
    others: &[DockRect],
    screens: &[DockRect],
    distance: f32,
) -> DockRect {
    let mut snapped = moving;
    if let Some(dx) = best_delta(x_candidates(&moving, others, screens, distance), distance) {
        snapped.x += dx;
    }
    if let Some(dy) = best_delta(y_candidates(&snapped, others, screens, distance), distance) {
        snapped.y += dy;
    }
    snapped
}

fn best_delta(candidates: Vec<f32>, distance: f32) -> Option<f32> {
    candidates
        .into_iter()
        .filter(|d| d.abs() <= distance)
        .min_by(|a, b| a.abs().total_cmp(&b.abs()))
}

/// Gap between two intervals (0 when they overlap).
fn interval_gap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (b0 - a1).max(a0 - b1).max(0.0)
}

fn x_candidates(m: &DockRect, others: &[DockRect], screens: &[DockRect], dist: f32) -> Vec<f32> {
    let mut out = Vec::new();
    for o in others {
        let v_gap = interval_gap(m.top(), m.bottom(), o.top(), o.bottom());
        if v_gap > dist {
            continue;
        }
        if v_gap <= TOUCH_EPSILON && m.overlaps_vertical(o) {
            // Side by side: put the windows edge to edge.
            out.push(o.right() - m.left());
            out.push(o.left() - m.right());
        }
        // Stacked (or nearly): line up left/right edges.
        out.push(o.left() - m.left());
        out.push(o.right() - m.right());
    }
    for s in screens {
        out.push(s.left() - m.left());
        out.push(s.right() - m.right());
    }
    out
}

fn y_candidates(m: &DockRect, others: &[DockRect], screens: &[DockRect], dist: f32) -> Vec<f32> {
    let mut out = Vec::new();
    for o in others {
        let h_gap = interval_gap(m.left(), m.right(), o.left(), o.right());
        if h_gap > dist {
            continue;
        }
        if h_gap <= TOUCH_EPSILON && m.overlaps_horizontal(o) {
            out.push(o.bottom() - m.top());
            out.push(o.top() - m.bottom());
        }
        out.push(o.top() - m.top());
        out.push(o.bottom() - m.bottom());
    }
    for s in screens {
        out.push(s.top() - m.top());
        out.push(s.bottom() - m.bottom());
    }
    out
}

/// True when two windows share an edge (within [`TOUCH_EPSILON`]) along an overlapping span.
pub fn touching(a: &DockRect, b: &DockRect) -> bool {
    let near = |p: f32, q: f32| (p - q).abs() <= TOUCH_EPSILON;
    let side = (near(a.right(), b.left()) || near(a.left(), b.right())) && a.overlaps_vertical(b);
    let stacked =
        (near(a.bottom(), b.top()) || near(a.top(), b.bottom())) && a.overlaps_horizontal(b);
    side || stacked
}

/// All windows connected to `root` through chains of touching edges (including `root`).
pub fn connected_group(root: WindowId, windows: &[(WindowId, DockRect)]) -> Vec<WindowId> {
    let mut group = vec![root];
    let mut i = 0;
    while i < group.len() {
        let current = group[i];
        if let Some((_, cur_rect)) = windows.iter().find(|(id, _)| *id == current) {
            for (id, rect) in windows {
                if !group.contains(id) && touching(cur_rect, rect) {
                    group.push(*id);
                }
            }
        }
        i += 1;
    }
    group
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: f32 = 10.0;

    fn main() -> DockRect {
        DockRect::new(100.0, 100.0, 275.0, 116.0)
    }

    #[test]
    fn snaps_flush_below_and_aligns_left_edge() {
        let moving = DockRect::new(106.0, 222.0, 275.0, 116.0);
        let s = snap_rect(moving, &[main()], &[], D);
        assert_eq!((s.x, s.y), (100.0, 216.0));
    }

    #[test]
    fn snaps_flush_to_the_right_side() {
        let moving = DockRect::new(381.0, 104.0, 275.0, 116.0);
        let s = snap_rect(moving, &[main()], &[], D);
        assert_eq!((s.x, s.y), (375.0, 100.0));
    }

    #[test]
    fn does_not_snap_beyond_distance() {
        let moving = DockRect::new(140.0, 240.0, 200.0, 100.0);
        assert_eq!(snap_rect(moving, &[main()], &[], D), moving);
    }

    #[test]
    fn far_windows_on_the_other_axis_do_not_attract() {
        // Left edge would align with main, but the window is 300 px below it.
        let moving = DockRect::new(104.0, 600.0, 275.0, 116.0);
        assert_eq!(snap_rect(moving, &[main()], &[], D).x, 104.0);
    }

    #[test]
    fn snaps_to_screen_edges() {
        let screen = DockRect::new(0.0, 0.0, 1440.0, 900.0);
        let moving = DockRect::new(7.0, 891.0 - 116.0, 275.0, 116.0);
        let s = snap_rect(moving, &[], &[screen], D);
        assert_eq!((s.x, s.bottom()), (0.0, 900.0));
    }

    #[test]
    fn picks_the_closest_candidate() {
        let a = DockRect::new(0.0, 0.0, 100.0, 100.0);
        let b = DockRect::new(0.0, 0.0, 104.0, 100.0);
        // Left of `moving` is 3 px from a.right and 1 px from b.right.
        let moving = DockRect::new(103.0, 0.0, 50.0, 100.0);
        assert_eq!(snap_rect(moving, &[a, b], &[], D).x, 104.0);
    }

    #[test]
    fn snapped_window_never_overlaps_its_target() {
        for dx in -9..=9 {
            for dy in -9..=9 {
                let moving = DockRect::new(100.0 + dx as f32, 216.0 + dy as f32, 275.0, 116.0);
                let s = snap_rect(moving, &[main()], &[], D);
                let m = main();
                let overlap = s.overlaps_horizontal(&m) && s.overlaps_vertical(&m);
                assert!(!overlap, "overlap at offset ({dx},{dy}) -> {s:?}");
            }
        }
    }

    #[test]
    fn touching_requires_shared_edge_and_overlap() {
        let below = DockRect::new(100.0, 216.0, 275.0, 116.0);
        let diagonal = DockRect::new(375.0, 216.0, 275.0, 116.0);
        assert!(touching(&main(), &below));
        assert!(!touching(&main(), &diagonal));
    }

    #[test]
    fn group_follows_chains_of_touching_windows() {
        let windows = [
            (WindowId::Main, main()),
            (
                WindowId::Equalizer,
                DockRect::new(100.0, 216.0, 275.0, 116.0),
            ),
            (
                WindowId::Playlist,
                DockRect::new(100.0, 332.0, 275.0, 232.0),
            ),
            (
                WindowId::DjConsole,
                DockRect::new(900.0, 900.0, 275.0, 116.0),
            ),
        ];
        let group = connected_group(WindowId::Main, &windows);
        assert_eq!(
            group,
            vec![WindowId::Main, WindowId::Equalizer, WindowId::Playlist]
        );
    }
}
