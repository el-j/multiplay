//! Pure geometry and state manager for magnetic window snapping and multi-window docking.
//! Inspired by the classic Winamp modular window system.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const DEFAULT_SNAP_THRESHOLD: f32 = 14.0;
pub const DEFAULT_TEAR_OFF_THRESHOLD: f32 = 24.0;

/// Identifier for distinct modular desktop windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowId {
    Main,
    Equalizer,
    Playlist,
    Settings,
    DjConsole,
}

/// 2D rectangular bounding box for floating windows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DockRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl DockRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    #[inline]
    pub fn left(&self) -> f32 {
        self.x
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    #[inline]
    pub fn top(&self) -> f32 {
        self.y
    }

    #[inline]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    #[inline]
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }

    /// Checks if two rects share an overlapping vertical interval.
    pub fn overlaps_vertical(&self, other: &Self) -> bool {
        self.top() < other.bottom() && self.bottom() > other.top()
    }

    /// Checks if two rects share an overlapping horizontal interval.
    pub fn overlaps_horizontal(&self, other: &Self) -> bool {
        self.left() < other.right() && self.right() > other.left()
    }
}

/// Docking bond edge relative to the host/anchor window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockEdge {
    Left,
    Right,
    Top,
    Bottom,
}

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
        windows.insert(WindowId::Main, DockRect::new(120.0, 100.0, 480.0, 190.0));
        windows.insert(WindowId::Equalizer, DockRect::new(120.0, 290.0, 480.0, 155.0));
        windows.insert(WindowId::Playlist, DockRect::new(120.0, 445.0, 480.0, 260.0));
        windows.insert(WindowId::Settings, DockRect::new(610.0, 100.0, 460.0, 400.0));
        windows.insert(WindowId::DjConsole, DockRect::new(120.0, 710.0, 480.0, 180.0));

        let mut visibility = HashMap::new();
        visibility.insert(WindowId::Main, true);
        visibility.insert(WindowId::Equalizer, true);
        visibility.insert(WindowId::Playlist, true);
        visibility.insert(WindowId::Settings, false);
        visibility.insert(WindowId::DjConsole, false);

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
    /// Returns Some((snapped_rect, target_id, edge)) if a snap occurred.
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

            // 1. Magnetic snap to Right edge of target (target | moving)
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

            // 2. Magnetic snap to Left edge of target (moving | target)
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

            // 3. Magnetic snap to Bottom edge of target (target / moving)
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

            // 4. Magnetic snap to Top edge of target (moving / target)
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

    pub fn get_rect(&self, id: WindowId) -> DockRect {
        self.windows.get(&id).copied().unwrap_or_else(|| DockRect::new(100.0, 100.0, 400.0, 200.0))
    }

    pub fn set_rect(&mut self, id: WindowId, rect: DockRect) {
        self.windows.insert(id, rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docking_snaps_to_bottom() {
        let mut dm = DockManager::new_empty();
        dm.windows.insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.set_visible(WindowId::Main, true);

        // Position Equalizer 5 pixels below Main (within 14px threshold)
        let eq_rect = DockRect::new(105.0, 305.0, 400.0, 150.0);
        let (snapped, dock) = dm.compute_snap(WindowId::Equalizer, eq_rect);

        assert_eq!(snapped.y, 300.0); // Flushed to Main bottom (100 + 200)
        assert_eq!(snapped.x, 100.0); // Flushed to Main left
        assert_eq!(dock, Some((WindowId::Main, DockEdge::Bottom)));
    }

    #[test]
    fn test_docking_snaps_to_right() {
        let mut dm = DockManager::new_empty();
        dm.windows.insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.set_visible(WindowId::Main, true);

        // Position Settings 8 pixels to the right of Main
        let settings_rect = DockRect::new(508.0, 105.0, 300.0, 200.0);
        let (snapped, dock) = dm.compute_snap(WindowId::Settings, settings_rect);

        assert_eq!(snapped.x, 500.0); // Flushed to Main right (100 + 400)
        assert_eq!(snapped.y, 100.0); // Flushed to Main top
        assert_eq!(dock, Some((WindowId::Main, DockEdge::Right)));
    }

    #[test]
    fn test_group_movement_propagates_to_docked_children() {
        let mut dm = DockManager::default();
        dm.windows.insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.windows.insert(WindowId::Equalizer, DockRect::new(100.0, 300.0, 400.0, 150.0));
        dm.dock(WindowId::Equalizer, WindowId::Main, DockEdge::Bottom);

        dm.move_window_group(WindowId::Main, 50.0, 30.0);

        assert_eq!(dm.get_rect(WindowId::Main).x, 150.0);
        assert_eq!(dm.get_rect(WindowId::Main).y, 130.0);
        assert_eq!(dm.get_rect(WindowId::Equalizer).x, 150.0);
        assert_eq!(dm.get_rect(WindowId::Equalizer).y, 330.0);
    }

    #[test]
    fn test_undock_allows_independent_movement() {
        let mut dm = DockManager::default();
        dm.windows.insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.windows.insert(WindowId::Equalizer, DockRect::new(100.0, 300.0, 400.0, 150.0));
        dm.dock(WindowId::Equalizer, WindowId::Main, DockEdge::Bottom);

        dm.undock(WindowId::Equalizer);
        dm.move_window_group(WindowId::Main, 50.0, 30.0);

        assert_eq!(dm.get_rect(WindowId::Main).x, 150.0);
        assert_eq!(dm.get_rect(WindowId::Equalizer).x, 100.0); // Did not move
    }

    #[test]
    fn test_layout_persistence() {
        let mut dm = DockManager::default();
        dm.set_rect(WindowId::Main, DockRect::new(200.0, 150.0, 500.0, 220.0));
        dm.set_visible(WindowId::Equalizer, false);
        dm.dock(WindowId::Playlist, WindowId::Main, DockEdge::Bottom);

        let json = serde_json::to_string(&dm).expect("Failed to serialize DockManager");
        let restored: DockManager = serde_json::from_str(&json).expect("Failed to deserialize DockManager");

        assert_eq!(restored.get_rect(WindowId::Main).x, 200.0);
        assert_eq!(restored.get_rect(WindowId::Main).y, 150.0);
        assert!(!restored.is_visible(WindowId::Equalizer));
        assert_eq!(restored.dock_parent.get(&WindowId::Playlist), Some(&(WindowId::Main, DockEdge::Bottom)));
    }
}

