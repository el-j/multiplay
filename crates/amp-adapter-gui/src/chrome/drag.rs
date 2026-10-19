//! App-driven window dragging in screen space.
//! The OS never drags our windows: a press on a title strip starts a [`DragSession`] and every
//! frame the session turns the pointer's screen position into snapped window positions for the
//! dragged window and the windows docked to it.

#![forbid(unsafe_code)]

use crate::docking::{connected_group, free_offset, snap_rect, DockRect, WindowId};

/// One title-bar drag, from press to release.
#[derive(Debug, Clone, PartialEq)]
pub struct DragSession {
    /// Window whose title bar was pressed.
    pub window: WindowId,
    /// Windows that move rigidly with it (always includes `window`), with their start rects.
    pub members: Vec<(WindowId, DockRect)>,
    press_pointer: (f32, f32),
}

impl DragSession {
    /// Starts a drag. Dragging `Main` takes every window docked to it along; dragging any other
    /// window pulls only that window away (it detaches from its group).
    pub fn start(
        window: WindowId,
        pointer_screen: (f32, f32),
        windows: &[(WindowId, DockRect)],
    ) -> Option<Self> {
        let group = if window == WindowId::Main {
            connected_group(window, windows)
        } else {
            vec![window]
        };
        let members: Vec<_> = group
            .into_iter()
            .filter_map(|id| windows.iter().find(|(w, _)| *w == id).copied())
            .collect();
        (!members.is_empty()).then_some(Self {
            window,
            members,
            press_pointer: pointer_screen,
        })
    }

    /// New rects for every member, given the pointer's current screen position.
    ///
    /// The dragged window snaps to non-member windows and to `screens`; members keep their
    /// offsets to it, so a docked group moves as one piece.
    pub fn positions(
        &self,
        pointer_screen: (f32, f32),
        windows: &[(WindowId, DockRect)],
        screens: &[DockRect],
        snap_distance: f32,
    ) -> Vec<(WindowId, DockRect)> {
        let dx = pointer_screen.0 - self.press_pointer.0;
        let dy = pointer_screen.1 - self.press_pointer.1;
        let others: Vec<DockRect> = windows
            .iter()
            .filter(|(id, _)| !self.members.iter().any(|(m, _)| m == id))
            .map(|(_, r)| *r)
            .collect();

        // The group's bounding box snaps, so a docked column stops flush against the screen
        // edge with whichever member reaches it.
        let start_bounds = bounds(self.members.iter().map(|(_, r)| *r));
        let mut moved_bounds = start_bounds;
        moved_bounds.translate(dx, dy);
        let snapped = snap_rect(moved_bounds, &others, screens, snap_distance);
        let (sx, sy) = (snapped.x - start_bounds.x, snapped.y - start_bounds.y);

        // Never land on another window: the group stops flush against the window it ran into.
        let rects: Vec<DockRect> = self.members.iter().map(|(_, r)| *r).collect();
        let (sx, sy) = free_offset(&rects, &others, (sx, sy));

        self.members
            .iter()
            .map(|(id, r)| {
                let mut moved = *r;
                moved.translate(sx, sy);
                (*id, moved)
            })
            .collect()
    }
}

fn bounds(rects: impl Iterator<Item = DockRect>) -> DockRect {
    let mut it = rects;
    let first = it.next().unwrap_or(DockRect::new(0.0, 0.0, 0.0, 0.0));
    let (mut l, mut t, mut r, mut b) = (first.left(), first.top(), first.right(), first.bottom());
    for rect in it {
        l = l.min(rect.left());
        t = t.min(rect.top());
        r = r.max(rect.right());
        b = b.max(rect.bottom());
    }
    DockRect::new(l, t, r - l, b - t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Vec<(WindowId, DockRect)> {
        vec![
            (WindowId::Main, DockRect::new(100.0, 100.0, 275.0, 116.0)),
            (
                WindowId::Equalizer,
                DockRect::new(100.0, 216.0, 275.0, 116.0),
            ),
            (
                WindowId::Playlist,
                DockRect::new(600.0, 100.0, 275.0, 232.0),
            ),
        ]
    }

    #[test]
    fn dragging_main_moves_its_docked_group_rigidly() {
        let w = layout();
        let s = DragSession::start(WindowId::Main, (150.0, 105.0), &w).unwrap();
        let moved = s.positions((190.0, 135.0), &w, &[], 10.0);
        let get = |id| moved.iter().find(|(w, _)| *w == id).unwrap().1;
        assert_eq!(
            (get(WindowId::Main).x, get(WindowId::Main).y),
            (140.0, 130.0)
        );
        assert_eq!(
            (get(WindowId::Equalizer).x, get(WindowId::Equalizer).y),
            (140.0, 246.0)
        );
        assert!(moved.iter().all(|(id, _)| *id != WindowId::Playlist));
    }

    #[test]
    fn dragging_a_child_detaches_only_that_window() {
        let w = layout();
        let s = DragSession::start(WindowId::Equalizer, (150.0, 220.0), &w).unwrap();
        assert_eq!(s.members.len(), 1);
        let moved = s.positions((150.0, 420.0), &w, &[], 10.0);
        assert_eq!(
            moved,
            vec![(
                WindowId::Equalizer,
                DockRect::new(100.0, 416.0, 275.0, 116.0)
            )]
        );
    }

    #[test]
    fn dragged_window_snaps_live_to_another_window() {
        let w = layout();
        // Playlist dragged so its left edge lands 6 px right of main's right edge.
        let s = DragSession::start(WindowId::Playlist, (700.0, 105.0), &w).unwrap();
        let moved = s.positions((481.0, 108.0), &w, &[], 10.0);
        assert_eq!(moved[0].1.x, 375.0);
        assert_eq!(moved[0].1.y, 100.0);
    }

    #[test]
    fn docked_group_snaps_to_the_screen_edge_as_one_piece() {
        let w = layout();
        let screen = DockRect::new(0.0, 0.0, 1440.0, 900.0);
        let s = DragSession::start(WindowId::Main, (150.0, 105.0), &w).unwrap();
        // Group bottom (332) dragged to 895: within 10 px of the screen bottom.
        let moved = s.positions((150.0, 105.0 + 563.0), &w, &[screen], 10.0);
        let eq = moved
            .iter()
            .find(|(id, _)| *id == WindowId::Equalizer)
            .unwrap()
            .1;
        assert_eq!(eq.bottom(), 900.0);
    }

    #[test]
    fn dragging_onto_another_window_stops_flush_against_it() {
        let w = layout();
        // Playlist dragged so it would cover the middle of the main window.
        let s = DragSession::start(WindowId::Playlist, (700.0, 105.0), &w).unwrap();
        let moved = s.positions((300.0, 140.0), &w, &[], 10.0);
        let main = w[0].1;
        assert!(
            !crate::docking::overlaps(&moved[0].1, &main),
            "{:?}",
            moved[0].1
        );
    }
}
