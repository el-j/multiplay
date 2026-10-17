//! Magnetic Multi-Window Docking and Snapping Window Manager for Winamp TUI.
//! Manages window geometries, magnetic snapping (threshold = 2 cells), and tear-off states.

#![forbid(unsafe_code)]

use ratatui::layout::Rect;
use std::collections::HashMap;

pub const SNAP_THRESHOLD_CELLS: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowId {
    MainConsole,
    Equalizer,
    Playlist,
    LibraryBrowser,
    VideoPlayer,
    SpectrumVisualizer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowSlot {
    pub id: WindowId,
    pub rect: Rect,
    pub visible: bool,
    pub docked_to: Option<WindowId>,
    pub z_index: usize,
}

#[derive(Debug, Clone)]
pub struct DockingWindowManager {
    windows: HashMap<WindowId, WindowSlot>,
    active_window: WindowId,
}

impl DockingWindowManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            windows: HashMap::new(),
            active_window: WindowId::MainConsole,
        };
        mgr.init_classic_layout();
        mgr
    }

    /// Initializes classic Winamp layout: Main faceplate (78x7), EQ below (78x6), Playlist below (78x12),
    /// and Library docked to the right side.
    pub fn init_classic_layout(&mut self) {
        // Main Console: top left
        self.windows.insert(
            WindowId::MainConsole,
            WindowSlot {
                id: WindowId::MainConsole,
                rect: Rect::new(0, 0, 78, 7),
                visible: true,
                docked_to: None,
                z_index: 0,
            },
        );

        // Equalizer: docked right under Main Console
        self.windows.insert(
            WindowId::Equalizer,
            WindowSlot {
                id: WindowId::Equalizer,
                rect: Rect::new(0, 7, 78, 6),
                visible: true,
                docked_to: Some(WindowId::MainConsole),
                z_index: 1,
            },
        );

        // Playlist: docked right under Equalizer
        self.windows.insert(
            WindowId::Playlist,
            WindowSlot {
                id: WindowId::Playlist,
                rect: Rect::new(0, 13, 78, 12),
                visible: true,
                docked_to: Some(WindowId::Equalizer),
                z_index: 2,
            },
        );

        // Library Browser: docked to the right of the stack
        self.windows.insert(
            WindowId::LibraryBrowser,
            WindowSlot {
                id: WindowId::LibraryBrowser,
                rect: Rect::new(78, 0, 42, 25),
                visible: true,
                docked_to: Some(WindowId::MainConsole),
                z_index: 3,
            },
        );

        // Detachable Video Player & Visualizer (floating / torn-off by default)
        self.windows.insert(
            WindowId::VideoPlayer,
            WindowSlot {
                id: WindowId::VideoPlayer,
                rect: Rect::new(120, 0, 60, 20),
                visible: false,
                docked_to: None,
                z_index: 4,
            },
        );

        self.windows.insert(
            WindowId::SpectrumVisualizer,
            WindowSlot {
                id: WindowId::SpectrumVisualizer,
                rect: Rect::new(0, 25, 78, 8),
                visible: false,
                docked_to: None,
                z_index: 5,
            },
        );
    }

    pub fn get_window(&self, id: WindowId) -> Option<&WindowSlot> {
        self.windows.get(&id)
    }

    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut WindowSlot> {
        self.windows.get_mut(&id)
    }

    pub fn set_visible(&mut self, id: WindowId, visible: bool) {
        if let Some(slot) = self.windows.get_mut(&id) {
            slot.visible = visible;
        }
    }

    pub fn active_window(&self) -> WindowId {
        self.active_window
    }

    pub fn set_active_window(&mut self, id: WindowId) {
        self.active_window = id;
    }

    /// Toggles tear-off: if docked, detaches and moves slightly; if detached, re-docks to nearest candidate.
    pub fn toggle_tear_off(&mut self, id: WindowId) {
        if let Some(slot) = self.windows.get_mut(&id) {
            if slot.docked_to.is_some() {
                // Detach
                slot.docked_to = None;
                slot.rect.x = slot.rect.x.saturating_add(4);
                slot.rect.y = slot.rect.y.saturating_add(2);
            } else {
                // Re-dock to main console
                slot.docked_to = Some(WindowId::MainConsole);
            }
        }
    }

    /// Moves a window and applies magnetic snapping to neighboring window borders.
    pub fn move_window_with_magnetic_snap(
        &mut self,
        target_id: WindowId,
        mut new_x: u16,
        mut new_y: u16,
    ) {
        let (width, height) = match self.windows.get(&target_id) {
            Some(w) => (w.rect.width, w.rect.height),
            None => return,
        };

        let target_bottom = new_y.saturating_add(height);
        let target_right = new_x.saturating_add(width);

        let mut candidates: Vec<(&WindowId, &WindowSlot)> = self.windows.iter().collect();
        candidates.sort_by_key(|(_, w)| w.z_index);

        let mut min_distance = i32::MAX;
        let mut snapped_dock: Option<WindowId> = None;

        for (&other_id, other) in candidates {
            if other_id == target_id || !other.visible {
                continue;
            }

            let other_top = other.rect.y;
            let other_bottom = other.rect.y.saturating_add(other.rect.height);
            let other_left = other.rect.x;
            let other_right = other.rect.x.saturating_add(other.rect.width);

            // 1. Top-to-Bottom edge magnetic snap: target top near other bottom
            let dist_tb = (new_y as i32 - other_bottom as i32).abs();
            if dist_tb <= SNAP_THRESHOLD_CELLS as i32 && dist_tb < min_distance {
                min_distance = dist_tb;
                new_y = other_bottom;
                if (new_x as i32 - other_left as i32).abs() <= SNAP_THRESHOLD_CELLS as i32 {
                    new_x = other_left;
                }
                snapped_dock = Some(other_id);
            }

            // 2. Bottom-to-Top edge magnetic snap: target bottom near other top
            let dist_bt = (target_bottom as i32 - other_top as i32).abs();
            if dist_bt <= SNAP_THRESHOLD_CELLS as i32 && dist_bt < min_distance {
                min_distance = dist_bt;
                new_y = other_top.saturating_sub(height);
                if (new_x as i32 - other_left as i32).abs() <= SNAP_THRESHOLD_CELLS as i32 {
                    new_x = other_left;
                }
                snapped_dock = Some(other_id);
            }

            // 3. Left-to-Right edge magnetic snap: target left near other right
            let dist_lr = (new_x as i32 - other_right as i32).abs();
            if dist_lr <= SNAP_THRESHOLD_CELLS as i32 && dist_lr < min_distance {
                min_distance = dist_lr;
                new_x = other_right;
                if (new_y as i32 - other_top as i32).abs() <= SNAP_THRESHOLD_CELLS as i32 {
                    new_y = other_top;
                }
                snapped_dock = Some(other_id);
            }

            // 4. Right-to-Left edge magnetic snap: target right near other left
            let dist_rl = (target_right as i32 - other_left as i32).abs();
            if dist_rl <= SNAP_THRESHOLD_CELLS as i32 && dist_rl < min_distance {
                min_distance = dist_rl;
                new_x = other_left.saturating_sub(width);
                if (new_y as i32 - other_top as i32).abs() <= SNAP_THRESHOLD_CELLS as i32 {
                    new_y = other_top;
                }
                snapped_dock = Some(other_id);
            }
        }

        if let Some(target) = self.windows.get_mut(&target_id) {
            target.rect.x = new_x;
            target.rect.y = new_y;
            if snapped_dock.is_some() {
                target.docked_to = snapped_dock;
            }
        }
    }
}

impl Default for DockingWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docking_classic_layout() {
        let mgr = DockingWindowManager::new();
        let main = mgr.get_window(WindowId::MainConsole).unwrap();
        let eq = mgr.get_window(WindowId::Equalizer).unwrap();
        let pl = mgr.get_window(WindowId::Playlist).unwrap();

        assert_eq!(main.rect, Rect::new(0, 0, 78, 7));
        assert_eq!(eq.rect, Rect::new(0, 7, 78, 6));
        assert_eq!(pl.rect, Rect::new(0, 13, 78, 12));
        assert_eq!(eq.docked_to, Some(WindowId::MainConsole));
    }

    #[test]
    fn test_magnetic_snapping() {
        let mut mgr = DockingWindowManager::new();

        // Move Equalizer close to Main Console bottom (y=7) but 1 cell off (y=8, x=1)
        mgr.move_window_with_magnetic_snap(WindowId::Equalizer, 1, 8);

        let eq = mgr.get_window(WindowId::Equalizer).unwrap();
        // Snapped exactly to y=7, x=0!
        assert_eq!(eq.rect.x, 0);
        assert_eq!(eq.rect.y, 7);
        assert_eq!(eq.docked_to, Some(WindowId::MainConsole));
    }

    #[test]
    fn test_tear_off_and_re_dock() {
        let mut mgr = DockingWindowManager::new();
        assert!(mgr
            .get_window(WindowId::Equalizer)
            .unwrap()
            .docked_to
            .is_some());

        // Tear off
        mgr.toggle_tear_off(WindowId::Equalizer);
        assert!(mgr
            .get_window(WindowId::Equalizer)
            .unwrap()
            .docked_to
            .is_none());

        // Re-dock
        mgr.toggle_tear_off(WindowId::Equalizer);
        assert_eq!(
            mgr.get_window(WindowId::Equalizer).unwrap().docked_to,
            Some(WindowId::MainConsole)
        );
    }
}
