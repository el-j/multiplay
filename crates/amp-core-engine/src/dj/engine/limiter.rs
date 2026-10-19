//! Master limiter (TASK-207): a stereo-linked lookahead brickwall that never lets a sample past
//! the ceiling.
//!
//! For each incoming frame the needed gain is `ceiling / peak`. The gain applied to a frame is
//! the average over `LOOKAHEAD` frames of minima taken over `2 × LOOKAHEAD` frames. Every value
//! in that average comes from a window that contains the frame being output, so the gain is
//! never above what that frame needs. The average makes the gain fall smoothly before a peak
//! arrives, which is why the audio is delayed by `LOOKAHEAD` frames. Recovery is gradual.

#![forbid(unsafe_code)]

/// Frames of lookahead (about 1.3 ms at 48 kHz).
const LOOKAHEAD: usize = 64;
const HOLD: usize = LOOKAHEAD * 2;
/// Highest output level (-0.18 dBFS).
pub const CEILING: f32 = 0.98;

/// Lookahead brickwall limiter state. Allocation-free after construction.
#[derive(Debug, Clone)]
pub(crate) struct Limiter {
    delay: Box<[f32]>,
    need: Box<[f32]>,
    mins: Box<[f32]>,
    window: MinWindow,
    at: usize,
    mins_sum: f32,
    gain: f32,
    release: f32,
}

impl Limiter {
    pub(crate) fn new(sample_rate: u32) -> Self {
        Self {
            delay: vec![0.0; LOOKAHEAD * 2].into_boxed_slice(),
            need: vec![1.0; HOLD].into_boxed_slice(),
            mins: vec![1.0; LOOKAHEAD].into_boxed_slice(),
            window: MinWindow::default(),
            at: 0,
            mins_sum: LOOKAHEAD as f32,
            gain: 1.0,
            // Back to unity within about 80 ms after the last over.
            release: 1.0 / (0.08 * sample_rate.max(1) as f32),
        }
    }

    /// Limits interleaved stereo `buf` in place. Returns the lowest gain applied.
    pub(crate) fn process(&mut self, buf: &mut [f32]) -> f32 {
        let mut lowest = 1.0f32;
        for f in buf.as_chunks_mut::<2>().0 {
            let peak = f[0].abs().max(f[1].abs());
            let need = if peak > CEILING { CEILING / peak } else { 1.0 };
            self.need[self.at % HOLD] = need;
            let held = self.window.push(self.at, need);
            let slot = self.at % LOOKAHEAD;
            self.mins_sum += held - self.mins[slot];
            self.mins[slot] = held;
            // The frame leaving the delay line needed `own`; capping by it makes the bound exact
            // despite rounding in the running average.
            let own = self.need[(self.at + HOLD - LOOKAHEAD) % HOLD];
            let target = (self.mins_sum / LOOKAHEAD as f32).min(own);
            self.gain = target.min(self.gain + self.release);
            let d = (self.at % LOOKAHEAD) * 2;
            let (out_l, out_r) = (self.delay[d], self.delay[d + 1]);
            self.delay[d] = f[0];
            self.delay[d + 1] = f[1];
            f[0] = out_l * self.gain;
            f[1] = out_r * self.gain;
            lowest = lowest.min(self.gain);
            self.at = self.at.wrapping_add(1);
        }
        // Keep the running sum from drifting.
        self.mins_sum = self.mins.iter().sum();
        lowest
    }
}

/// Minimum of the last `HOLD` values in O(1) per value (a monotonic queue in fixed storage).
#[derive(Debug, Clone)]
struct MinWindow {
    at: Box<[usize]>,
    value: Box<[f32]>,
    head: usize,
    len: usize,
}

impl Default for MinWindow {
    fn default() -> Self {
        Self {
            at: vec![0; HOLD + 1].into_boxed_slice(),
            value: vec![0.0; HOLD + 1].into_boxed_slice(),
            head: 0,
            len: 0,
        }
    }
}

impl MinWindow {
    /// Adds `v` seen at step `now` and returns the minimum of the last `HOLD` steps (values
    /// before the first push count as 1).
    fn push(&mut self, now: usize, v: f32) -> f32 {
        let cap = self.at.len();
        while self.len > 0 && self.value[(self.head + self.len - 1) % cap] >= v {
            self.len -= 1;
        }
        let tail = (self.head + self.len) % cap;
        self.at[tail] = now;
        self.value[tail] = v;
        self.len += 1;
        while now.wrapping_sub(self.at[self.head]) >= HOLD {
            self.head = (self.head + 1) % cap;
            self.len -= 1;
        }
        if now + 1 < HOLD {
            self.value[self.head].min(1.0)
        } else {
            self.value[self.head]
        }
    }
}

/// Delays interleaved stereo by the limiter's lookahead, so a bus without the limiter (the cue
/// bus) stays time-aligned with the limited master in the headphone mix.
#[derive(Debug, Clone)]
pub(crate) struct LookaheadDelay {
    buf: Box<[f32]>,
    at: usize,
}

impl Default for LookaheadDelay {
    fn default() -> Self {
        Self {
            buf: vec![0.0; LOOKAHEAD * 2].into_boxed_slice(),
            at: 0,
        }
    }
}

impl LookaheadDelay {
    pub(crate) fn process(&mut self, buf: &mut [f32]) {
        for f in buf.as_chunks_mut::<2>().0 {
            let d = self.at * 2;
            (f[0], self.buf[d]) = (self.buf[d], f[0]);
            (f[1], self.buf[d + 1]) = (self.buf[d + 1], f[1]);
            self.at = (self.at + 1) % LOOKAHEAD;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amp: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = amp * (2.0 * std::f32::consts::PI * 220.0 * i as f32 / 48_000.0).sin();
                [v, v * 0.7]
            })
            .collect()
    }

    #[test]
    fn dj_mixer_limiter_stops_overs_on_a_plus_6_db_mix() {
        let mut lim = Limiter::new(48_000);
        let mut buf = tone(2.0, 48_000);
        for block in buf.chunks_mut(512) {
            lim.process(block);
        }
        let peak = buf.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        assert!(peak <= CEILING + 1e-6, "peak {peak}");
        assert!(peak > 0.9, "still loud: {peak}");
    }

    #[test]
    fn dj_mixer_limiter_catches_a_sudden_spike() {
        let mut lim = Limiter::new(48_000);
        let mut buf = tone(0.3, 4_800);
        buf[4_000] = 3.0;
        buf[4_001] = -3.0;
        lim.process(&mut buf);
        let peak = buf.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        assert!(peak <= CEILING + 1e-6, "spike passed at {peak}");
    }

    #[test]
    fn dj_mixer_limiter_window_minimum_matches_a_scan() {
        let mut w = MinWindow::default();
        let values: Vec<f32> = (0..2_000)
            .map(|i| ((i * 7919) % 1_000) as f32 / 1_000.0)
            .collect();
        for (i, &v) in values.iter().enumerate() {
            let lo = i.saturating_sub(HOLD - 1);
            let scan = values[lo..=i].iter().copied().fold(1.0f32, f32::min);
            assert_eq!(w.push(i, v), scan, "step {i}");
        }
    }

    #[test]
    fn dj_mixer_cue_delay_matches_the_limiter_delay() {
        let mut lim = Limiter::new(48_000);
        let mut delay = LookaheadDelay::default();
        let input = tone(0.5, 2_000);
        let (mut a, mut b) = (input.clone(), input);
        lim.process(&mut a);
        delay.process(&mut b);
        assert_eq!(a, b);
    }

    #[test]
    fn dj_mixer_limiter_is_transparent_below_the_ceiling() {
        let mut lim = Limiter::new(48_000);
        let input = tone(0.5, 4_800);
        let mut buf = input.clone();
        assert_eq!(lim.process(&mut buf), 1.0);
        // Only delayed, otherwise untouched.
        assert_eq!(&buf[LOOKAHEAD * 2..], &input[..input.len() - LOOKAHEAD * 2]);
    }
}
