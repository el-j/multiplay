//! Loops and hot cues of one deck (TASK-206), in source frames.
//!
//! Seams are click-free: over the last few milliseconds before the loop end the deck blends
//! into the audio that leads up to the loop start, so at the wrap the two sides already match
//! and the first hit inside the loop still plays at full strength.

#![forbid(unsafe_code)]

/// Hot cue slots per track.
pub const HOT_CUES: usize = 8;
/// Length of the seam blend.
const SEAM_SECS: f64 = 0.005;

/// Loop and cue state of a deck.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct LoopState {
    /// Loop start and end in source frames (kept after exiting, for reloop).
    pub range: Option<(f64, f64)>,
    pub active: bool,
    /// Loop-in point waiting for its loop-out (A of an A-B loop).
    pub pending_a: Option<f64>,
    /// Where the track would be without the roll, while a loop roll runs.
    pub shadow: Option<f64>,
    /// Hot cues in seconds.
    pub cues: [Option<f64>; HOT_CUES],
}

impl LoopState {
    /// Clears loops (a new track keeps no loop; its cues are set by the control side).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Starts looping `start..end` (source frames); ignored when the range is empty.
    pub fn set(&mut self, start: f64, end: f64) {
        if end > start {
            self.range = Some((start, end));
            self.active = true;
        }
    }

    /// The active loop range.
    pub fn active_range(&self) -> Option<(f64, f64)> {
        self.range.filter(|_| self.active)
    }

    /// Wraps `position` back to the loop start when it reached the loop end. Returns true when
    /// it wrapped.
    pub fn wrap(&self, position: &mut f64) -> bool {
        match self.active_range() {
            Some((s, e)) if *position >= e => {
                *position = s + (*position - e) % (e - s);
                true
            }
            _ => false,
        }
    }

    /// Inside the seam blend before the loop end: the matching position before the loop start
    /// and the weight (0 → 1) to give it.
    pub fn seam(&self, position: f64, rate: f64) -> Option<(f64, f32)> {
        let (s, e) = self.active_range()?;
        let fade = (SEAM_SECS * rate).min((e - s) / 4.0);
        if fade < 1.0 || position < e - fade || position >= e {
            return None;
        }
        let x = (position - (e - fade)) / fade;
        let w = 0.5 - 0.5 * (std::f64::consts::PI * x).cos();
        Some((position - (e - s), w as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_loops_wrap_keeps_the_overshoot() {
        let mut l = LoopState::default();
        l.set(100.0, 200.0);
        let mut p = 203.5;
        assert!(l.wrap(&mut p));
        assert_eq!(p, 103.5);
        let mut inside = 150.0;
        assert!(!l.wrap(&mut inside));
    }

    #[test]
    fn dj_loops_seam_blends_only_just_before_the_end() {
        let mut l = LoopState::default();
        l.set(48_000.0, 96_000.0);
        assert!(l.seam(90_000.0, 48_000.0).is_none());
        let (other, w) = l.seam(95_880.0, 48_000.0).unwrap();
        assert_eq!(other, 47_880.0);
        assert!((w - 0.5).abs() < 1e-6, "halfway through the 240-frame seam");
        assert!(l.seam(96_000.0, 48_000.0).is_none());
    }

    #[test]
    fn dj_loops_ignore_empty_ranges_and_keep_the_range_after_exit() {
        let mut l = LoopState::default();
        l.set(10.0, 10.0);
        assert!(l.active_range().is_none());
        l.set(10.0, 20.0);
        l.active = false;
        assert_eq!(l.range, Some((10.0, 20.0)));
        assert!(l.active_range().is_none());
    }
}
