//! Window layout persistence: positions, sizes, visibility, shade and double size survive a
//! restart. Docks are not stored; they are rebuilt from touching edges on load.

#![forbid(unsafe_code)]

use crate::docking::{
    connected_group, free_offset, rescale_layout, DockManager, DockRect, WindowId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Current on-disk schema. Files without a version are the pre-1.0 `DockManager` dump.
pub const LAYOUT_VERSION: u32 = 1;
const ALL: [WindowId; 6] = [
    WindowId::Main,
    WindowId::Equalizer,
    WindowId::Playlist,
    WindowId::Settings,
    WindowId::DjConsole,
    WindowId::Remote,
];

/// One window's saved state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub id: WindowId,
    /// At 1x (zoom 1.0), whatever the zoom was when saved.
    pub rect: DockRect,
    pub visible: bool,
    pub shaded: bool,
    /// Height to roll back down to when the window is shaded.
    #[serde(default)]
    pub restore_height: Option<f32>,
}

/// Everything needed to reopen the windows as they were.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    pub version: u32,
    /// 2.0 in double size.
    pub zoom: f32,
    pub windows: Vec<WindowState>,
}

impl LayoutSnapshot {
    /// Captures the layout, re-expressed at 1x so files do not depend on the zoom.
    pub fn capture(dm: &DockManager, shade_restore: &HashMap<WindowId, f32>, zoom: f32) -> Self {
        let rects: Vec<_> = ALL
            .iter()
            .filter_map(|id| dm.windows.get(id).map(|r| (*id, *r)))
            .collect();
        let windows = rescale_layout(&rects, WindowId::Main, zoom, 1.0)
            .into_iter()
            .map(|(id, rect)| WindowState {
                id,
                rect,
                visible: dm.is_visible(id),
                shaded: dm.is_shaded(id),
                restore_height: shade_restore.get(&id).copied(),
            })
            .collect();
        Self {
            version: LAYOUT_VERSION,
            zoom,
            windows,
        }
    }

    /// Restores the snapshot at 1x; the saved zoom is applied separately by the caller.
    pub fn apply(&self, dm: &mut DockManager, shade_restore: &mut HashMap<WindowId, f32>) {
        shade_restore.clear();
        for w in &self.windows {
            dm.set_rect(w.id, w.rect);
            // Main is always shown; the settings overlay never reopens by itself.
            let visible = (w.visible && w.id != WindowId::Settings) || w.id == WindowId::Main;
            dm.set_visible(w.id, visible);
            dm.shaded.insert(w.id, w.shaded);
            if let (true, Some(h)) = (w.shaded, w.restore_height) {
                shade_restore.insert(w.id, h);
            }
        }
        dm.rebuild_docks_from_contact();
    }

    fn from_legacy(dm: &DockManager) -> Self {
        let mut s = Self::capture(dm, &HashMap::new(), 1.0);
        // The old format never stored restore heights, so nothing comes back shaded.
        for w in &mut s.windows {
            w.shaded = false;
        }
        s
    }
}

/// Writes `snapshot` to `file` atomically (temp file, then rename).
pub fn save_layout(snapshot: &LayoutSnapshot, file: &Path) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(snapshot).map_err(std::io::Error::other)?;
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, file)
}

/// Reads the layout from `file`, migrating the pre-1.0 format. `None` when missing, unreadable
/// or from a newer version.
pub fn load_layout(file: &Path) -> Option<LayoutSnapshot> {
    let json = std::fs::read_to_string(file).ok()?;
    if let Ok(s) = serde_json::from_str::<LayoutSnapshot>(&json) {
        return (s.version <= LAYOUT_VERSION && s.zoom.is_finite() && s.zoom > 0.0).then_some(s);
    }
    let legacy = serde_json::from_str::<DockManager>(&json).ok()?;
    Some(LayoutSnapshot::from_legacy(&legacy))
}

/// Moves windows that are not on any of `screens` (a monitor that was unplugged, a changed
/// resolution) back onto the first screen. Docked groups move as one piece and stay docked;
/// groups that already show their top-left corner on a screen stay where they are.
pub fn fit_on_screen(
    windows: &[(WindowId, DockRect)],
    screens: &[DockRect],
) -> Vec<(WindowId, DockRect)> {
    let Some(home) = screens.first().copied() else {
        return windows.to_vec();
    };
    let mut remaining = windows.to_vec();
    let mut placed: Vec<(WindowId, DockRect)> = Vec::new();
    while let Some(&(first, _)) = remaining.first() {
        let seed = if remaining.iter().any(|(w, _)| *w == WindowId::Main) {
            WindowId::Main
        } else {
            first
        };
        let ids = connected_group(seed, &remaining);
        let (members, rest): (Vec<_>, Vec<_>) =
            remaining.into_iter().partition(|(w, _)| ids.contains(w));
        remaining = rest;
        let rects: Vec<DockRect> = members.iter().map(|(_, r)| *r).collect();
        let bounds = bounding(&rects);
        let shown = screens.iter().any(|s| {
            bounds.x >= s.left()
                && bounds.x < s.right()
                && bounds.y >= s.top()
                && bounds.y < s.bottom()
        });
        let want = if shown {
            (0.0, 0.0)
        } else {
            (
                shift_into(bounds.x, bounds.width, home.x, home.width),
                shift_into(bounds.y, bounds.height, home.y, home.height),
            )
        };
        let occupied: Vec<DockRect> = placed.iter().map(|(_, r)| *r).collect();
        let (dx, dy) = free_offset(&rects, &occupied, want);
        for (id, mut r) in members {
            r.translate(dx, dy);
            placed.push((id, r));
        }
    }
    windows
        .iter()
        .filter_map(|(id, _)| placed.iter().find(|(w, _)| w == id).copied())
        .collect()
}

fn bounding(rects: &[DockRect]) -> DockRect {
    let l = rects.iter().map(|r| r.left()).fold(f32::MAX, f32::min);
    let t = rects.iter().map(|r| r.top()).fold(f32::MAX, f32::min);
    let r = rects.iter().map(|r| r.right()).fold(f32::MIN, f32::max);
    let b = rects.iter().map(|r| r.bottom()).fold(f32::MIN, f32::max);
    DockRect::new(l, t, r - l, b - t)
}

/// Offset that brings a span `[pos, pos + len]` inside `[start, start + room]` (its start edge
/// wins when it is longer than the room).
fn shift_into(pos: f32, len: f32, start: f32, room: f32) -> f32 {
    let target = pos.min(start + room - len).max(start);
    target - pos
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file() -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("multiplay-layout-{}", uuid::Uuid::new_v4()))
            .join("window_layout.json")
    }

    #[test]
    fn layout_roundtrips_visibility_shade_and_double_size() {
        let mut dm = DockManager::default();
        dm.set_visible(WindowId::DjConsole, true);
        dm.set_rect(
            WindowId::Equalizer,
            DockRect::new(120.0, 310.0, 480.0, 14.0),
        );
        dm.shaded.insert(WindowId::Equalizer, true);
        let restore = HashMap::from([(WindowId::Equalizer, 200.0)]);
        let snap = LayoutSnapshot::capture(&dm, &restore, 1.0);
        let file = temp_file();
        save_layout(&snap, &file).unwrap();
        let loaded = load_layout(&file).unwrap();
        assert_eq!(loaded, snap);

        let (mut back, mut back_restore) = (DockManager::new_empty(), HashMap::new());
        loaded.apply(&mut back, &mut back_restore);
        assert!(back.is_visible(WindowId::DjConsole));
        assert!(back.is_shaded(WindowId::Equalizer));
        assert_eq!(back_restore.get(&WindowId::Equalizer), Some(&200.0));
        assert_eq!(
            back.get_rect(WindowId::Playlist),
            dm.get_rect(WindowId::Playlist)
        );
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
    }

    #[test]
    fn layout_never_reopens_the_settings_overlay() {
        let mut dm = DockManager::default();
        dm.set_visible(WindowId::Settings, true);
        let snap = LayoutSnapshot::capture(&dm, &HashMap::new(), 1.0);
        let mut back = DockManager::new_empty();
        snap.apply(&mut back, &mut HashMap::new());
        assert!(!back.is_visible(WindowId::Settings));
        assert!(back.is_visible(WindowId::Main));
    }

    #[test]
    fn layout_is_stored_at_1x_in_double_size() {
        let dm = DockManager::default();
        let at_1x = LayoutSnapshot::capture(&dm, &HashMap::new(), 1.0);
        let mut doubled = dm.clone();
        let rects: Vec<_> = dm.windows.iter().map(|(i, r)| (*i, *r)).collect();
        for (id, r) in rescale_layout(&rects, WindowId::Main, 1.0, 2.0) {
            doubled.set_rect(id, r);
        }
        let at_2x = LayoutSnapshot::capture(&doubled, &HashMap::new(), 2.0);
        assert_eq!(at_2x.zoom, 2.0);
        assert_eq!(at_2x.windows, at_1x.windows);
    }

    #[test]
    fn layout_migrates_the_old_dock_manager_file() {
        let file = temp_file();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let mut old = DockManager::default();
        old.set_rect(WindowId::Main, DockRect::new(40.0, 50.0, 480.0, 210.0));
        std::fs::write(&file, serde_json::to_string(&old).unwrap()).unwrap();
        let s = load_layout(&file).expect("migrated");
        assert_eq!(s.version, LAYOUT_VERSION);
        assert_eq!(s.windows[0].rect, DockRect::new(40.0, 50.0, 480.0, 210.0));
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
    }

    #[test]
    fn layout_from_a_newer_version_or_garbage_is_ignored() {
        let file = temp_file();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, r#"{"version":99,"zoom":1.0,"windows":[]}"#).unwrap();
        assert!(load_layout(&file).is_none());
        std::fs::write(&file, "not json").unwrap();
        assert!(load_layout(&file).is_none());
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
    }

    #[test]
    fn layout_on_an_unplugged_second_monitor_reopens_fully_visible_and_docked() {
        let screen = DockRect::new(0.0, 0.0, 1440.0, 900.0);
        let saved = vec![
            (WindowId::Main, DockRect::new(2000.0, 100.0, 480.0, 210.0)),
            (
                WindowId::Equalizer,
                DockRect::new(2000.0, 310.0, 480.0, 200.0),
            ),
            (
                WindowId::Playlist,
                DockRect::new(2000.0, 510.0, 480.0, 300.0),
            ),
        ];
        let out = fit_on_screen(&saved, &[screen]);
        for (id, r) in &out {
            assert!(
                r.left() >= 0.0 && r.right() <= 1440.0 && r.top() >= 0.0 && r.bottom() <= 900.0,
                "{id:?} at {r:?} is not fully on screen"
            );
        }
        assert_eq!(
            out[1].1.top(),
            out[0].1.bottom(),
            "EQ still docked below main"
        );
        assert_eq!(
            out[2].1.top(),
            out[1].1.bottom(),
            "playlist still docked below EQ"
        );
        assert_eq!(out[0].1.left(), out[2].1.left());
    }

    #[test]
    fn layout_already_on_a_screen_is_left_alone() {
        let screens = [
            DockRect::new(0.0, 0.0, 1440.0, 900.0),
            DockRect::new(1440.0, 0.0, 1920.0, 1080.0),
        ];
        let saved = vec![
            (WindowId::Main, DockRect::new(120.0, 100.0, 480.0, 210.0)),
            (
                WindowId::Playlist,
                DockRect::new(1600.0, 200.0, 480.0, 300.0),
            ),
        ];
        assert_eq!(fit_on_screen(&saved, &screens), saved);
    }

    #[test]
    fn layout_groups_brought_back_do_not_land_on_each_other() {
        let screen = DockRect::new(0.0, 0.0, 1440.0, 900.0);
        let saved = vec![
            (WindowId::Main, DockRect::new(1200.0, 100.0, 480.0, 210.0)),
            (
                WindowId::Playlist,
                DockRect::new(3000.0, 100.0, 480.0, 300.0),
            ),
        ];
        let out = fit_on_screen(&saved, &[screen]);
        assert!(!crate::docking::overlaps(&out[0].1, &out[1].1), "{out:?}");
    }
}
