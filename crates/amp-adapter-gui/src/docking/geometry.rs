//! Window identification and bounding box geometry for magnetic docking.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Classic magnetic snap distance in points (spec §4; configurable later).
pub const DEFAULT_SNAP_THRESHOLD: f32 = 10.0;
pub const DEFAULT_TEAR_OFF_THRESHOLD: f32 = 24.0;

/// Identifier for distinct modular desktop windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowId {
    Main,
    Equalizer,
    Playlist,
    Settings,
    DjConsole,
    /// Remote & Sharing: phone requests, paired devices, pairing (TASK-214).
    Remote,
    /// Universal Omnibox Search Window (TASK-153): YouTube, Bandcamp & Library.
    Search,
    /// Library browser: artists, albums, tracks, recently added (TASK-172).
    Library,
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
        Self {
            x,
            y,
            width,
            height,
        }
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
