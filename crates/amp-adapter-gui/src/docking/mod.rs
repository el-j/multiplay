//! Pure geometry and state manager for magnetic window snapping and multi-window docking.
//! Inspired by the classic Winamp modular window system.

#![forbid(unsafe_code)]

pub mod geometry;
pub mod manager;

pub use geometry::{
    DockEdge, DockRect, WindowId, DEFAULT_SNAP_THRESHOLD, DEFAULT_TEAR_OFF_THRESHOLD,
};
pub use manager::DockManager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docking_snaps_to_bottom() {
        let mut dm = DockManager::new_empty();
        dm.windows
            .insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.set_visible(WindowId::Main, true);

        let eq_rect = DockRect::new(105.0, 305.0, 400.0, 150.0);
        let (snapped, dock) = dm.compute_snap(WindowId::Equalizer, eq_rect);

        assert_eq!(snapped.y, 300.0);
        assert_eq!(snapped.x, 100.0);
        assert_eq!(dock, Some((WindowId::Main, DockEdge::Bottom)));
    }

    #[test]
    fn test_docking_snaps_to_right() {
        let mut dm = DockManager::new_empty();
        dm.windows
            .insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.set_visible(WindowId::Main, true);

        let settings_rect = DockRect::new(508.0, 105.0, 300.0, 200.0);
        let (snapped, dock) = dm.compute_snap(WindowId::Settings, settings_rect);

        assert_eq!(snapped.x, 500.0);
        assert_eq!(snapped.y, 100.0);
        assert_eq!(dock, Some((WindowId::Main, DockEdge::Right)));
    }

    #[test]
    fn test_group_movement_propagates_to_docked_children() {
        let mut dm = DockManager::default();
        dm.windows
            .insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.windows.insert(
            WindowId::Equalizer,
            DockRect::new(100.0, 300.0, 400.0, 150.0),
        );
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
        dm.windows
            .insert(WindowId::Main, DockRect::new(100.0, 100.0, 400.0, 200.0));
        dm.windows.insert(
            WindowId::Equalizer,
            DockRect::new(100.0, 300.0, 400.0, 150.0),
        );
        dm.dock(WindowId::Equalizer, WindowId::Main, DockEdge::Bottom);

        dm.undock(WindowId::Equalizer);
        dm.move_window_group(WindowId::Main, 50.0, 30.0);

        assert_eq!(dm.get_rect(WindowId::Main).x, 150.0);
        assert_eq!(dm.get_rect(WindowId::Equalizer).x, 100.0);
    }

    #[test]
    fn test_layout_persistence() {
        let mut dm = DockManager::default();
        dm.set_rect(WindowId::Main, DockRect::new(200.0, 150.0, 500.0, 220.0));
        dm.set_visible(WindowId::Equalizer, false);
        dm.dock(WindowId::Playlist, WindowId::Main, DockEdge::Bottom);

        let json = serde_json::to_string(&dm).expect("Failed to serialize DockManager");
        let restored: DockManager =
            serde_json::from_str(&json).expect("Failed to deserialize DockManager");

        assert_eq!(restored.get_rect(WindowId::Main).x, 200.0);
        assert_eq!(restored.get_rect(WindowId::Main).y, 150.0);
        assert!(!restored.is_visible(WindowId::Equalizer));
        assert_eq!(
            restored.dock_parent.get(&WindowId::Playlist),
            Some(&(WindowId::Main, DockEdge::Bottom))
        );
    }
}
