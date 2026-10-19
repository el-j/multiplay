//! DJ waveforms (TASK-211): stacked scrolling three-band detail waveforms for both decks with
//! beat markers at a fixed playhead, a phase meter, and per-deck overviews with playhead, hot
//! cues and loop region (click to seek). Everything is drawn from the cached analysis; nothing
//! is recomputed per frame.
//!
//! The detail lanes share one axis of *heard* time: a deck's track time is divided by its speed.
//! When two decks are synced their beats meet in heard time, so their beat markers line up.

#![forbid(unsafe_code)]

use amp_domain::TrackAnalysis;
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, Vec2};

/// Heard seconds shown across a detail lane.
pub const WINDOW_SECS: f64 = 6.0;

/// What a detail lane needs about its deck.
#[derive(Debug, Clone, Copy)]
pub struct LaneView<'a> {
    /// The deck's analysis.
    pub analysis: &'a TrackAnalysis,
    /// Playhead in track seconds.
    pub position: f64,
    /// Playing speed (1 = original tempo).
    pub speed: f64,
    /// Deck colour.
    pub color: Color32,
}

/// Horizontal position of track time `t` in a lane `rect` whose centre is the playhead.
pub fn x_of(t: f64, view: &LaneView<'_>, rect: Rect) -> f32 {
    let heard = (t - view.position) / view.speed.max(0.01);
    rect.center().x + (heard / WINDOW_SECS) as f32 * rect.width()
}

/// Beat markers visible in `rect`: `(x, is_downbeat)`.
pub fn beat_xs<'a>(view: &'a LaneView<'a>, rect: Rect) -> impl Iterator<Item = (f32, bool)> + 'a {
    let period = view.analysis.beat_period().unwrap_or(0.0);
    let half = WINDOW_SECS / 2.0 * view.speed.max(0.01);
    let anchor = view.analysis.first_downbeat_secs;
    let bar = i64::from(view.analysis.beats_per_bar.max(1));
    let (first, last) = if period > 0.0 {
        (
            ((view.position - half - anchor) / period).floor() as i64,
            ((view.position + half - anchor) / period).ceil() as i64,
        )
    } else {
        (1, 0)
    };
    (first..=last).map(move |n| {
        let t = anchor + n as f64 * period;
        (x_of(t, view, rect), n.rem_euclid(bar) == 0)
    })
}

/// Paints one detail lane: low, mid and high band levels mirrored around the centre line,
/// beat markers, and the playhead.
pub fn paint_lane(painter: &Painter, rect: Rect, view: &LaneView<'_>) {
    painter.rect_filled(rect, 2.0, Color32::from_rgb(8, 10, 14));
    let a = view.analysis;
    let mid_y = rect.center().y;
    let half_h = rect.height() / 2.0 - 1.0;
    let rate = f64::from(a.detail_rate.max(1.0));
    let colors = [
        view.color.gamma_multiply(0.45),
        view.color.gamma_multiply(0.75),
        Color32::from_rgb(235, 240, 250),
    ];
    let mut x = rect.left();
    while x < rect.right() {
        let heard = f64::from((x - rect.center().x) / rect.width()) * WINDOW_SECS;
        let t = view.position + heard * view.speed;
        let col = (t * rate).floor();
        if col >= 0.0 {
            if let Some(levels) = a.detail.get(col as usize) {
                // Wider bands drawn first, brighter highs on top.
                for (band, color) in levels.iter().zip(colors) {
                    let h = f32::from(*band) / 255.0 * half_h;
                    if h >= 0.5 {
                        painter.line_segment(
                            [Pos2::new(x, mid_y - h), Pos2::new(x, mid_y + h)],
                            Stroke::new(1.0_f32, color),
                        );
                    }
                }
            }
        }
        x += 1.0;
    }
    for (bx, downbeat) in beat_xs(view, rect) {
        if bx >= rect.left() && bx <= rect.right() {
            let (len, alpha) = if downbeat {
                (half_h, 200)
            } else {
                (half_h * 0.35, 120)
            };
            let c = Color32::from_white_alpha(alpha);
            painter.line_segment(
                [Pos2::new(bx, rect.top()), Pos2::new(bx, rect.top() + len)],
                Stroke::new(1.0_f32, c),
            );
            painter.line_segment(
                [
                    Pos2::new(bx, rect.bottom() - len),
                    Pos2::new(bx, rect.bottom()),
                ],
                Stroke::new(1.0_f32, c),
            );
        }
    }
    let cx = rect.center().x;
    painter.line_segment(
        [Pos2::new(cx, rect.top()), Pos2::new(cx, rect.bottom())],
        Stroke::new(1.5_f32, Color32::from_rgb(255, 60, 60)),
    );
}

/// Paints the phase meter: how far deck B's beat is from deck A's, -½ … +½ beat.
pub fn paint_phase_meter(painter: &Painter, rect: Rect, error_beats: Option<f64>) {
    painter.rect_filled(rect, 1.0, Color32::from_rgb(20, 24, 30));
    let cx = rect.center().x;
    painter.line_segment(
        [Pos2::new(cx, rect.top()), Pos2::new(cx, rect.bottom())],
        Stroke::new(1.0_f32, Color32::from_gray(90)),
    );
    if let Some(e) = error_beats {
        let e = e.clamp(-0.5, 0.5) as f32;
        let x = cx + e * rect.width();
        let aligned = e.abs() < 0.02;
        let color = if aligned {
            Color32::from_rgb(0, 230, 120)
        } else {
            Color32::from_rgb(255, 190, 40)
        };
        let bar = Rect::from_two_pos(
            Pos2::new(cx, rect.top() + 1.0),
            Pos2::new(x, rect.bottom() - 1.0),
        );
        painter.rect_filled(bar, 0.0, color);
    }
}

/// Hot cue and loop marks for an overview.
#[derive(Debug, Clone, Copy, Default)]
pub struct Marks<'a> {
    /// Hot cue times in seconds (empty slots are `None`).
    pub cues: &'a [Option<f64>],
    /// Loop `(start, end)` in seconds.
    pub loop_range: Option<(f64, f64)>,
    /// Whether the loop is active.
    pub loop_active: bool,
}

/// Draws a whole-track overview with the playhead, hot cues and loop region. Returns the time
/// (seconds) the user clicked, to seek there.
pub fn overview(
    ui: &mut egui::Ui,
    rect: Rect,
    analysis: Option<&TrackAnalysis>,
    position: f64,
    duration: f64,
    color: Color32,
    marks: Marks<'_>,
) -> Option<f64> {
    let response = ui.allocate_rect(rect, egui::Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_rgb(8, 10, 14));
    let to_x =
        |t: f64| rect.left() + (t / duration.max(0.001)).clamp(0.0, 1.0) as f32 * rect.width();
    if let Some((s, e)) = marks.loop_range {
        let fill = if marks.loop_active {
            Color32::from_rgba_unmultiplied(0, 200, 120, 60)
        } else {
            Color32::from_rgba_unmultiplied(120, 120, 120, 40)
        };
        painter.rect_filled(
            Rect::from_x_y_ranges(to_x(s)..=to_x(e), rect.y_range()),
            0.0,
            fill,
        );
    }
    if let Some(a) = analysis.filter(|a| !a.overview.is_empty()) {
        let n = a.overview.len();
        let mid = rect.center().y;
        let half = rect.height() / 2.0 - 1.0;
        let played = to_x(position);
        let mut x = rect.left();
        while x < rect.right() {
            let i = (((x - rect.left()) / rect.width()) * n as f32) as usize;
            let h = f32::from(a.overview[i.min(n - 1)]) / 255.0 * half;
            let c = if x <= played {
                color.gamma_multiply(0.4)
            } else {
                color
            };
            painter.line_segment(
                [Pos2::new(x, mid - h), Pos2::new(x, mid + h)],
                Stroke::new(1.0_f32, c),
            );
            x += 1.0;
        }
    }
    for (slot, cue) in marks.cues.iter().enumerate() {
        if let Some(t) = cue {
            let x = to_x(*t);
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0_f32, CUE_COLORS[slot % CUE_COLORS.len()]),
            );
        }
    }
    let px = to_x(position);
    painter.line_segment(
        [Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())],
        Stroke::new(1.5_f32, Color32::WHITE),
    );
    response
        .clicked()
        .then(|| response.interact_pointer_pos())
        .flatten()
        .map(|p| f64::from((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0) * duration)
}

/// Hot cue colours by slot.
pub const CUE_COLORS: [Color32; 8] = [
    Color32::from_rgb(255, 80, 80),
    Color32::from_rgb(255, 160, 40),
    Color32::from_rgb(250, 230, 60),
    Color32::from_rgb(80, 220, 100),
    Color32::from_rgb(60, 200, 230),
    Color32::from_rgb(90, 120, 255),
    Color32::from_rgb(190, 100, 255),
    Color32::from_rgb(255, 110, 200),
];

/// Size of a detail lane for a given width (kept here so tests and the console agree).
pub fn lane_size(width: f32, height: f32) -> Vec2 {
    Vec2::new(width, height.max(12.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::ANALYSIS_VERSION;

    fn analysis(bpm: f64, anchor: f64) -> TrackAnalysis {
        TrackAnalysis {
            version: ANALYSIS_VERSION,
            duration_secs: 300.0,
            bpm: Some(bpm),
            first_downbeat_secs: anchor,
            beats_per_bar: 4,
            overview: vec![128; 1_024],
            detail: vec![[100, 80, 60]; 30_000],
            detail_rate: 100.0,
            loudness_lufs: None,
        }
    }

    fn lane() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 40.0))
    }

    #[test]
    fn dj_waveform_playhead_is_the_lane_centre() {
        let a = analysis(120.0, 0.0);
        let v = LaneView {
            analysis: &a,
            position: 42.0,
            speed: 1.0,
            color: Color32::RED,
        };
        assert_eq!(x_of(42.0, &v, lane()), 300.0);
        assert_eq!(
            x_of(45.0, &v, lane()),
            600.0,
            "half the window is three heard seconds"
        );
    }

    #[test]
    fn dj_waveform_synced_decks_show_beats_at_the_same_pixels() {
        // Deck A at 120 BPM on a beat; deck B at 124 BPM slowed to 120 and in phase.
        let (a, b) = (analysis(120.0, 0.25), analysis(124.0, 0.6));
        let va = LaneView {
            analysis: &a,
            position: 0.25 + 40.0 * 0.5,
            speed: 1.0,
            color: Color32::RED,
        };
        let vb = LaneView {
            analysis: &b,
            position: 0.6 + 33.0 * 60.0 / 124.0,
            speed: 120.0 / 124.0,
            color: Color32::BLUE,
        };
        let xa: Vec<f32> = beat_xs(&va, lane()).map(|(x, _)| x).collect();
        let xb: Vec<f32> = beat_xs(&vb, lane()).map(|(x, _)| x).collect();
        for x in xa.iter().filter(|x| (0.0..=600.0).contains(*x)) {
            let nearest = xb.iter().map(|y| (y - x).abs()).fold(f32::MAX, f32::min);
            assert!(
                nearest < 0.5,
                "A's beat at {x} has no B beat within half a pixel"
            );
        }
    }

    #[test]
    fn dj_waveform_downbeats_repeat_every_bar() {
        let a = analysis(120.0, 0.0);
        let v = LaneView {
            analysis: &a,
            position: 10.0,
            speed: 1.0,
            color: Color32::RED,
        };
        let beats: Vec<(f32, bool)> = beat_xs(&v, lane()).collect();
        let down: Vec<f32> = beats.iter().filter(|b| b.1).map(|b| b.0).collect();
        assert!(down.len() >= 2);
        assert!(
            (down[1] - down[0] - 200.0).abs() < 0.01,
            "a bar is 2 s = 200 px"
        );
        let xs: Vec<f32> = beats.iter().map(|b| b.0).collect();
        assert!(
            xs[0] <= 0.0 && *xs.last().unwrap() >= 600.0,
            "markers cover the lane"
        );
    }
}
