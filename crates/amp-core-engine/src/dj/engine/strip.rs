//! A deck's channel strip (TASK-207): a 3-band isolator EQ with kills and a one-knob filter.
//!
//! The EQ splits the signal with 4th-order Linkwitz-Riley crossovers at 250 Hz and 2.5 kHz.
//! The low band also passes the 2.5 kHz all-pass, so at 0 dB the three bands sum to a flat
//! magnitude response. Each band gain runs from a kill (silence) to +6 dB.
//!
//! The filter knob runs from -1 (low-pass down to 60 Hz) through 0 (off) to +1 (high-pass up
//! to 8 kHz). It is a 12 dB/octave Butterworth whose cutoff moves without resetting its state.

#![forbid(unsafe_code)]

use crate::dsp::BiquadFilter;

/// Low/mid crossover in Hz.
pub const LOW_MID_HZ: f32 = 250.0;
/// Mid/high crossover in Hz.
pub const MID_HIGH_HZ: f32 = 2_500.0;
const Q: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// Knob values closer to 0 than this leave the filter off.
const FILTER_DEAD_ZONE: f32 = 0.02;

/// Cutoff of the one-knob filter: low-pass for `knob < 0`, high-pass for `knob > 0`.
pub fn filter_cutoff(knob: f32) -> f32 {
    let k = knob.clamp(-1.0, 1.0);
    if k < 0.0 {
        20_000.0 * (60.0f32 / 20_000.0).powf(-k)
    } else {
        20.0 * (8_000.0f32 / 20.0).powf(k)
    }
}

/// 4th-order Linkwitz-Riley section: two identical Butterworth biquads.
#[derive(Debug, Clone, Copy)]
struct Lr4([BiquadFilter; 2]);

impl Lr4 {
    fn low(rate: f32, hz: f32) -> Self {
        Self([BiquadFilter::new_lowpass(rate, hz, Q); 2])
    }
    fn high(rate: f32, hz: f32) -> Self {
        Self([BiquadFilter::new_highpass(rate, hz, Q); 2])
    }
    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.0[0].process_sample(x);
        self.0[1].process_sample(y)
    }
}

/// One audio channel of the 3-band split.
#[derive(Debug, Clone, Copy)]
struct Bands {
    low: Lr4,
    low_ap_lo: Lr4,
    low_ap_hi: Lr4,
    rest: Lr4,
    mid: Lr4,
    high: Lr4,
}

impl Bands {
    fn new(rate: f32) -> Self {
        Self {
            low: Lr4::low(rate, LOW_MID_HZ),
            low_ap_lo: Lr4::low(rate, MID_HIGH_HZ),
            low_ap_hi: Lr4::high(rate, MID_HIGH_HZ),
            rest: Lr4::high(rate, LOW_MID_HZ),
            mid: Lr4::low(rate, MID_HIGH_HZ),
            high: Lr4::high(rate, MID_HIGH_HZ),
        }
    }

    #[inline]
    fn run(&mut self, x: f32, g: [f32; 3]) -> f32 {
        let lo = self.low.run(x);
        let lo = self.low_ap_lo.run(lo) + self.low_ap_hi.run(lo);
        let rest = self.rest.run(x);
        g[0] * lo + g[1] * self.mid.run(rest) + g[2] * self.high.run(rest)
    }
}

/// The one-knob filter of one audio channel pair.
#[derive(Debug, Clone, Copy)]
struct KnobFilter {
    filters: [BiquadFilter; 2],
    knob: f32,
    rate: f32,
}

impl KnobFilter {
    fn new(rate: f32) -> Self {
        Self {
            filters: [BiquadFilter::new_lowpass(rate, 20_000.0, Q); 2],
            knob: 0.0,
            rate,
        }
    }

    fn set(&mut self, knob: f32) {
        if (knob - self.knob).abs() < 1e-4 {
            return;
        }
        let crossed = (knob < 0.0) != (self.knob < 0.0);
        self.knob = knob;
        if knob.abs() < FILTER_DEAD_ZONE {
            return;
        }
        let hz = filter_cutoff(knob);
        let design = if knob < 0.0 {
            BiquadFilter::new_lowpass(self.rate, hz, Q)
        } else {
            BiquadFilter::new_highpass(self.rate, hz, Q)
        };
        for f in &mut self.filters {
            if crossed {
                *f = design;
            } else {
                f.retune(&design);
            }
        }
    }

    #[inline]
    fn run(&mut self, l: f32, r: f32) -> (f32, f32) {
        if self.knob.abs() < FILTER_DEAD_ZONE {
            return (l, r);
        }
        (
            self.filters[0].process_sample(l),
            self.filters[1].process_sample(r),
        )
    }
}

/// EQ and filter state of one deck. Allocation-free.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChannelStrip {
    bands: [Bands; 2],
    filter: KnobFilter,
    gains: [f32; 3],
}

impl ChannelStrip {
    pub(crate) fn new(rate: f32) -> Self {
        Self {
            bands: [Bands::new(rate); 2],
            filter: KnobFilter::new(rate),
            gains: [1.0; 3],
        }
    }

    /// Runs interleaved stereo `buf` through the EQ (band gains ramp to `gains` across the
    /// block) and the filter at `knob`.
    pub(crate) fn process(&mut self, buf: &mut [f32], gains: [f32; 3], knob: f32) {
        self.filter.set(knob);
        let frames = (buf.len() / 2).max(1) as f32;
        let from = self.gains;
        for (i, f) in buf.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let t = (i + 1) as f32 / frames;
            let g = [0, 1, 2].map(|b| from[b] + (gains[b] - from[b]) * t);
            let l = self.bands[0].run(f[0], g);
            let r = self.bands[1].run(f[1], g);
            (f[0], f[1]) = self.filter.run(l, r);
        }
        self.gains = gains;
    }
}

/// Linear gain for an EQ band: `db` from -40 to +6 dB, or 0 when killed.
pub fn band_gain(db: f32, kill: bool) -> f32 {
    if kill {
        0.0
    } else {
        10.0f32.powf(db.clamp(-40.0, 6.0) / 20.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    /// Steady-state gain in dB of the strip for a sine at `hz`.
    fn response(gains: [f32; 3], knob: f32, hz: f32) -> f32 {
        let mut strip = ChannelStrip::new(RATE);
        strip.gains = gains;
        let n = 48_000;
        let mut buf: Vec<f32> = (0..n)
            .flat_map(|i| {
                let v = (2.0 * std::f32::consts::PI * hz * i as f32 / RATE).sin();
                [v, v]
            })
            .collect();
        for block in buf.chunks_mut(512) {
            strip.process(block, gains, knob);
        }
        let tail = &buf[n..];
        let rms = (tail.iter().map(|v| v * v).sum::<f32>() / tail.len() as f32).sqrt();
        20.0 * (rms / std::f32::consts::FRAC_1_SQRT_2).log10()
    }

    #[test]
    fn dj_mixer_eq_is_flat_at_0_db() {
        for hz in [
            30.0, 100.0, 250.0, 700.0, 1_000.0, 2_500.0, 6_000.0, 16_000.0,
        ] {
            let db = response([1.0; 3], 0.0, hz);
            assert!(db.abs() < 0.5, "{hz} Hz: {db:.2} dB");
        }
    }

    #[test]
    fn dj_mixer_eq_bands_boost_and_kill_their_range() {
        let up = band_gain(6.0, false);
        assert!((response([1.0, up, 1.0], 0.0, 1_000.0) - 6.0).abs() < 0.5);
        assert!((response([up, 1.0, 1.0], 0.0, 50.0) - 6.0).abs() < 0.5);
        assert!((response([1.0, 1.0, up], 0.0, 12_000.0) - 6.0).abs() < 0.5);
        assert!(response([0.0, 1.0, 1.0], 0.0, 50.0) < -30.0, "low kill");
        assert!(response([1.0, 0.0, 1.0], 0.0, 900.0) < -25.0, "mid kill");
        assert!(
            response([1.0, 1.0, 0.0], 0.0, 12_000.0) < -30.0,
            "high kill"
        );
        assert!(
            response([0.0, 1.0, 1.0], 0.0, 1_000.0).abs() < 0.5,
            "kill keeps other bands"
        );
    }

    #[test]
    fn dj_mixer_filter_cuts_3_db_at_its_cutoff() {
        for knob in [-0.6f32, -0.3, 0.3, 0.6] {
            let fc = filter_cutoff(knob);
            let at = response([1.0; 3], knob, fc);
            assert!(
                (at + 3.01).abs() < 0.5,
                "knob {knob}: {at:.2} dB at {fc:.0} Hz"
            );
            let pass = if knob < 0.0 { fc / 8.0 } else { fc * 8.0 };
            let ok = response([1.0; 3], knob, pass.clamp(30.0, 18_000.0));
            assert!(ok.abs() < 0.5, "knob {knob}: pass band {ok:.2} dB");
        }
        assert!(response([1.0; 3], 0.01, 50.0).abs() < 0.5, "centre is off");
    }

    #[test]
    fn dj_mixer_filter_ends_reach_60_hz_and_8_khz() {
        assert!((filter_cutoff(-1.0) - 60.0).abs() < 0.5);
        assert!((filter_cutoff(1.0) - 8_000.0).abs() < 1.0);
        assert!(filter_cutoff(-0.5) > filter_cutoff(-0.9));
    }
}
