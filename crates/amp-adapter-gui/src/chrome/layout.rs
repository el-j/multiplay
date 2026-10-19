//! Classic title-bar geometry and hit-testing, independent of egui.
//! Positions follow the classic 275 px faceplate: a 14 px strip with minimize, shade and close
//! buttons (9 x 9) at its right end. Everything scales with the double-size factor.

#![forbid(unsafe_code)]

/// Height of the title strip at 1x.
pub const TITLE_HEIGHT: f32 = 14.0;
const BUTTON: f32 = 9.0;
const BUTTON_TOP: f32 = 3.0;
const CLOSE_FROM_RIGHT: f32 = 11.0;
const SHADE_FROM_RIGHT: f32 = 21.0;
const MINIMIZE_FROM_RIGHT: f32 = 31.0;

/// Axis-aligned rectangle in window-local points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Bounds {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// What a pointer position on the title strip refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeHit {
    Minimize,
    Shade,
    Close,
    /// The draggable part of the strip.
    Title,
    /// Below the strip: window content.
    Content,
}

/// Title-strip layout for a window of a given width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromeLayout {
    pub title: Bounds,
    pub minimize: Option<Bounds>,
    pub shade: Option<Bounds>,
    pub close: Bounds,
}

impl ChromeLayout {
    /// `scale` is the double-size factor (1.0 or 2.0). Windows without a minimize/shade button
    /// (the settings overlay) pass `full = false`.
    pub fn new(width: f32, scale: f32, full: bool) -> Self {
        let button = |from_right: f32| Bounds {
            x: width - from_right * scale,
            y: BUTTON_TOP * scale,
            w: BUTTON * scale,
            h: BUTTON * scale,
        };
        Self {
            title: Bounds {
                x: 0.0,
                y: 0.0,
                w: width,
                h: TITLE_HEIGHT * scale,
            },
            minimize: full.then(|| button(MINIMIZE_FROM_RIGHT)),
            shade: full.then(|| button(SHADE_FROM_RIGHT)),
            close: button(CLOSE_FROM_RIGHT),
        }
    }

    pub fn hit(&self, px: f32, py: f32) -> ChromeHit {
        if !self.title.contains(px, py) {
            return ChromeHit::Content;
        }
        if self.close.contains(px, py) {
            ChromeHit::Close
        } else if self.shade.is_some_and(|b| b.contains(px, py)) {
            ChromeHit::Shade
        } else if self.minimize.is_some_and(|b| b.contains(px, py)) {
            ChromeHit::Minimize
        } else {
            ChromeHit::Title
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_button_positions_at_1x() {
        let l = ChromeLayout::new(275.0, 1.0, true);
        assert_eq!(l.hit(268.0, 7.0), ChromeHit::Close);
        assert_eq!(l.hit(258.0, 7.0), ChromeHit::Shade);
        assert_eq!(l.hit(248.0, 7.0), ChromeHit::Minimize);
        assert_eq!(l.hit(100.0, 7.0), ChromeHit::Title);
        assert_eq!(l.hit(100.0, 20.0), ChromeHit::Content);
    }

    #[test]
    fn gaps_between_buttons_drag_the_window() {
        let l = ChromeLayout::new(275.0, 1.0, true);
        assert_eq!(l.hit(263.0, 7.0), ChromeHit::Title);
        assert_eq!(l.hit(268.0, 1.0), ChromeHit::Title);
    }

    #[test]
    fn everything_scales_at_2x() {
        let l = ChromeLayout::new(550.0, 2.0, true);
        assert_eq!(l.title.h, 28.0);
        assert_eq!(l.hit(536.0, 14.0), ChromeHit::Close);
        assert_eq!(l.hit(516.0, 14.0), ChromeHit::Shade);
        assert_eq!(l.hit(100.0, 30.0), ChromeHit::Content);
    }

    #[test]
    fn overlay_layout_has_only_close() {
        let l = ChromeLayout::new(400.0, 1.0, false);
        assert!(l.minimize.is_none() && l.shade.is_none());
        assert_eq!(l.hit(372.0, 7.0), ChromeHit::Title);
        assert_eq!(l.hit(393.0, 7.0), ChromeHit::Close);
    }
}
