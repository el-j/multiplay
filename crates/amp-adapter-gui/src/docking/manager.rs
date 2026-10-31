//! DockManager orchestrator for window positioning, snapping, and group translation.

#![forbid(unsafe_code)]

use super::geometry::{
    DockEdge, DockRect, WindowId, DEFAULT_SNAP_THRESHOLD, DEFAULT_TEAR_OFF_THRESHOLD,
};
use super::snap::{connected_group, touching, TOUCH_EPSILON};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Manages docking connections and magnetic snapping across all windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockManager {
    pub windows: HashMap<WindowId, DockRect>,
    pub visibility: HashMap<WindowId, bool>,
    pub shaded: HashMap<WindowId, bool>,
    /// Dock relationships: key is docked to value (parent).
    pub dock_parent: HashMap<WindowId, (WindowId, DockEdge)>,
    pub snap_threshold: f32,
    pub tear_off_threshold: f32,
}

impl Default for DockManager {
    fn default() -> Self {
        let mut windows = HashMap::new();
        // Classic Winamp layout coordinates
        windows.insert(WindowId::Main, DockRect::new(120.0, 100.0, 480.0, 210.0));
        windows.insert(
            WindowId::Equalizer,
            DockRect::new(120.0, 310.0, 480.0, 200.0),
        );
        windows.insert(
            WindowId::Playlist,
            DockRect::new(120.0, 510.0, 480.0, 300.0),
        );
        windows.insert(
            WindowId::Settings,
            DockRect::new(610.0, 100.0, 520.0, 420.0),
        );
        windows.insert(
            WindowId::DjConsole,
            DockRect::new(610.0, 530.0, 480.0, 220.0),
        );
        windows.insert(WindowId::Remote, DockRect::new(610.0, 310.0, 360.0, 320.0));
        windows.insert(WindowId::Search, DockRect::new(610.0, 100.0, 480.0, 360.0));

        let mut visibility = HashMap::new();
        visibility.insert(WindowId::Main, true);
        visibility.insert(WindowId::Equalizer, true);
        visibility.insert(WindowId::Playlist, true);
        visibility.insert(WindowId::Settings, false);
        visibility.insert(WindowId::DjConsole, false);
        visibility.insert(WindowId::Remote, false);
        visibility.insert(WindowId::Search, false);

        let mut dock_parent = HashMap::new();
        dock_parent.insert(WindowId::Equalizer, (WindowId::Main, DockEdge::Bottom));
        dock_parent.insert(WindowId::Playlist, (WindowId::Equalizer, DockEdge::Bottom));

        Self {
            windows,
            visibility,
            shaded: HashMap::new(),
            dock_parent,
            snap_threshold: DEFAULT_SNAP_THRESHOLD,
            tear_off_threshold: DEFAULT_TEAR_OFF_THRESHOLD,
        }
    }
}

impl DockManager {
    /// Attempts to snap `moving_rect` against a list of stationary target rectangles.
    pub fn compute_snap(
        &self,
        moving_id: WindowId,
        moving_rect: DockRect,
    ) -> (DockRect, Option<(WindowId, DockEdge)>) {
        let mut snapped = moving_rect;
        let mut dock_info = None;

        for (&target_id, &target_rect) in &self.windows {
            if target_id == moving_id || !self.is_visible(target_id) {
                continue;
            }

            // Magnetic snap to Right edge
            if moving_rect.overlaps_vertical(&target_rect)
                && (moving_rect.left() - target_rect.right()).abs() <= self.snap_threshold
            {
                snapped.x = target_rect.right();
                if (snapped.top() - target_rect.top()).abs() <= self.snap_threshold {
                    snapped.y = target_rect.top();
                }
                dock_info = Some((target_id, DockEdge::Right));
                break;
            }

            // Magnetic snap to Left edge
            if moving_rect.overlaps_vertical(&target_rect)
                && (moving_rect.right() - target_rect.left()).abs() <= self.snap_threshold
            {
                snapped.x = target_rect.left() - snapped.width;
                if (snapped.top() - target_rect.top()).abs() <= self.snap_threshold {
                    snapped.y = target_rect.top();
                }
                dock_info = Some((target_id, DockEdge::Left));
                break;
            }

            // Magnetic snap to Bottom edge
            if moving_rect.overlaps_horizontal(&target_rect)
                && (moving_rect.top() - target_rect.bottom()).abs() <= self.snap_threshold
            {
                snapped.y = target_rect.bottom();
                if (snapped.left() - target_rect.left()).abs() <= self.snap_threshold {
                    snapped.x = target_rect.left();
                }
                dock_info = Some((target_id, DockEdge::Bottom));
                break;
            }

            // Magnetic snap to Top edge
            if moving_rect.overlaps_horizontal(&target_rect)
                && (moving_rect.bottom() - target_rect.top()).abs() <= self.snap_threshold
            {
                snapped.y = target_rect.top() - snapped.height;
                if (snapped.left() - target_rect.left()).abs() <= self.snap_threshold {
                    snapped.x = target_rect.left();
                }
                dock_info = Some((target_id, DockEdge::Top));
                break;
            }
        }

        (snapped, dock_info)
    }

    /// Moves a window and all transitively docked child windows by (dx, dy).
    pub fn move_window_group(&mut self, window_id: WindowId, dx: f32, dy: f32) {
        let mut group = HashSet::new();
        self.collect_docked_group(window_id, &mut group);

        for id in group {
            if let Some(rect) = self.windows.get_mut(&id) {
                rect.translate(dx, dy);
            }
        }
    }

    /// Recursively collects all child windows docked directly or indirectly to `root`.
    fn collect_docked_group(&self, root: WindowId, group: &mut HashSet<WindowId>) {
        group.insert(root);
        for (&child, &(parent, _)) in &self.dock_parent {
            if parent == root && !group.contains(&child) {
                self.collect_docked_group(child, group);
            }
        }
    }

    /// Undocks a window from its parent.
    pub fn undock(&mut self, window_id: WindowId) {
        self.dock_parent.remove(&window_id);
    }

    /// Sets docking bond between child and parent.
    pub fn dock(&mut self, child: WindowId, parent: WindowId, edge: DockEdge) {
        if child != parent {
            self.dock_parent.insert(child, (parent, edge));
        }
    }

    pub fn is_visible(&self, id: WindowId) -> bool {
        self.visibility.get(&id).copied().unwrap_or(false)
    }

    pub fn set_visible(&mut self, id: WindowId, visible: bool) {
        self.visibility.insert(id, visible);
    }

    pub fn toggle_visible(&mut self, id: WindowId) {
        let cur = self.is_visible(id);
        self.set_visible(id, !cur);
    }

    pub fn is_shaded(&self, id: WindowId) -> bool {
        self.shaded.get(&id).copied().unwrap_or(false)
    }

    pub fn toggle_shade(&mut self, id: WindowId) {
        let cur = self.is_shaded(id);
        self.shaded.insert(id, !cur);
    }

    pub fn new_empty() -> Self {
        Self {
            windows: HashMap::new(),
            visibility: HashMap::new(),
            shaded: HashMap::new(),
            dock_parent: HashMap::new(),
            snap_threshold: DEFAULT_SNAP_THRESHOLD,
            tear_off_threshold: DEFAULT_TEAR_OFF_THRESHOLD,
        }
    }

    /// Visible windows that take part in docking (the settings overlay never does).
    pub fn dockable_windows(&self) -> Vec<(WindowId, DockRect)> {
        let mut out: Vec<_> = self
            .windows
            .iter()
            .filter(|(id, _)| {
                **id != WindowId::Settings && **id != WindowId::Search && self.is_visible(**id)
            })
            .map(|(id, r)| (*id, *r))
            .collect();
        out.sort_by_key(|(id, _)| *id as u8);
        out
    }

    /// Re-derives dock bonds from touching edges, walking outward from the main window.
    pub fn rebuild_docks_from_contact(&mut self) {
        let windows = self.dockable_windows();
        let group = connected_group(WindowId::Main, &windows);
        let rect = |id: WindowId| windows.iter().find(|(w, _)| *w == id).map(|(_, r)| *r);
        self.dock_parent.clear();
        for (i, &child) in group.iter().enumerate().skip(1) {
            let Some(c) = rect(child) else { continue };
            let parent = group[..i]
                .iter()
                .copied()
                .find(|p| rect(*p).is_some_and(|r| touching(&r, &c)));
            if let (Some(parent), Some(pr)) = (parent, parent.and_then(rect)) {
                self.dock_parent.insert(child, (parent, edge_of(&pr, &c)));
            }
        }
    }

    pub fn get_rect(&self, id: WindowId) -> DockRect {
        self.windows
            .get(&id)
            .copied()
            .unwrap_or_else(|| DockRect::new(100.0, 100.0, 400.0, 200.0))
    }

    pub fn set_rect(&mut self, id: WindowId, rect: DockRect) {
        self.windows.insert(id, rect);
    }
}

/// Side of `parent` that `child` is attached to.
fn edge_of(parent: &DockRect, child: &DockRect) -> DockEdge {
    if (child.top() - parent.bottom()).abs() <= TOUCH_EPSILON {
        DockEdge::Bottom
    } else if (child.bottom() - parent.top()).abs() <= TOUCH_EPSILON {
        DockEdge::Top
    } else if (child.left() - parent.right()).abs() <= TOUCH_EPSILON {
        DockEdge::Right
    } else {
        DockEdge::Left
    }
}
