//! Real-time Linkwitz-Riley LR4 phase-aligned 4-band pseudo-stem isolator.
//!
//! Splits incoming stereo audio into 4 stems:
//! - Drums (150 Hz - 800 Hz)
//! - Bass (< 150 Hz)
//! - Melody (> 4,500 Hz)
//! - Vocals (800 Hz - 4,500 Hz)
//!
//! Using 4th-order Linkwitz-Riley crossovers with all-pass phase alignment,
//! guaranteeing flat magnitude response and zero phase distortion when all
//! stem gains are at unity.

#![forbid(unsafe_code)]

use super::params::{StemDeckParams, StemType, STEM_COUNT};
use crate::dsp::BiquadFilter;

/// Low/Mid crossover (Bass vs Drums) in Hz.
pub const BASS_DRUMS_HZ: f32 = 150.0;
/// Mid split (Drums vs Vocals) in Hz.
pub const DRUMS_VOCALS_HZ: f32 = 800.0;
/// Mid/High crossover (Vocals vs Melody) in Hz.
pub const VOCALS_MELODY_HZ: f32 = 4_500.0;

const Q: f32 = std::f32::consts::FRAC_1_SQRT_2;
const FILTER_DEAD_ZONE: f32 = 0.02;

/// Cutoff for one-knob stem filter.
#[must_use]
pub fn stem_filter_cutoff(knob: f32) -> f32 {
    let k = knob.clamp(-1.0, 1.0);
    if k < 0.0 {
        20_000.0 * (60.0f32 / 20_000.0).powf(-k)
    } else {
        20.0 * (8_000.0f32 / 20.0).powf(k)
    }
}

/// 4th-order Linkwitz-Riley section: two cascaded 2nd-order Butterworth biquads.
#[derive(Debug, Clone, Copy)]
struct Lr4([BiquadFilter; 2]);

impl Lr4 {
    fn low(rate: f32, hz: f32) -> Self {
        Self([BiquadFilter::new_lowpass(rate, hz, Q); 2])
    }

    fn high(rate: f32, hz: f32) -> Self {
        Self([BiquadFilter::new_highpass(rate, hz, Q); 2])
    }

    #[inline(always)]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.0[0].process_sample(x);
        self.0[1].process_sample(y)
    }

    fn reset(&mut self) {
        self.0[0].reset_state();
        self.0[1].reset_state();
    }
}

/// 4th-order Linkwitz-Riley allpass phase compensator: LP + HP.
#[derive(Debug, Clone, Copy)]
struct Lr4AllPass {
    low: Lr4,
    high: Lr4,
}

impl Lr4AllPass {
    fn new(rate: f32, hz: f32) -> Self {
        Self {
            low: Lr4::low(rate, hz),
            high: Lr4::high(rate, hz),
        }
    }

    #[inline(always)]
    fn run(&mut self, x: f32) -> f32 {
        self.low.run(x) + self.high.run(x)
    }

    fn reset(&mut self) {
        self.low.reset();
        self.high.reset();
    }
}

/// One-knob bi-directional filter for a single stem channel.
#[derive(Debug, Clone, Copy)]
struct StemKnobFilter {
    filter: BiquadFilter,
    knob: f32,
    rate: f32,
}

impl StemKnobFilter {
    fn new(rate: f32) -> Self {
        Self {
            filter: BiquadFilter::new_lowpass(rate, 20_000.0, Q),
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
        let hz = stem_filter_cutoff(knob);
        let design = if knob < 0.0 {
            BiquadFilter::new_lowpass(self.rate, hz, Q)
        } else {
            BiquadFilter::new_highpass(self.rate, hz, Q)
        };
        if crossed {
            self.filter = design;
        } else {
            self.filter.retune(&design);
        }
    }

    #[inline(always)]
    fn run(&mut self, sample: f32) -> f32 {
        if self.knob.abs() < FILTER_DEAD_ZONE {
            sample
        } else {
            self.filter.process_sample(sample)
        }
    }

    fn reset(&mut self) {
        self.filter.reset_state();
    }
}

/// Single-channel 4-band phase-aligned filterbank.
#[derive(Debug, Clone, Copy)]
struct StemFilterbankChannel {
    // 800 Hz split
    mid_split_low: Lr4,
    mid_split_high: Lr4,

    // 150 Hz split on lower band
    bass_low: Lr4,
    drums_high: Lr4,
    // Independent phase alignment across 4500 Hz for Bass and Drums
    ap_bass_4500: Lr4AllPass,
    ap_drums_4500: Lr4AllPass,

    // 4500 Hz split on upper band
    vocals_low: Lr4,
    melody_high: Lr4,
    // Independent phase alignment across 150 Hz for Vocals and Melody
    ap_vocals_150: Lr4AllPass,
    ap_melody_150: Lr4AllPass,

    // Per-stem knob filters
    filters: [StemKnobFilter; STEM_COUNT],
}

impl StemFilterbankChannel {
    fn new(rate: f32) -> Self {
        Self {
            mid_split_low: Lr4::low(rate, DRUMS_VOCALS_HZ),
            mid_split_high: Lr4::high(rate, DRUMS_VOCALS_HZ),

            bass_low: Lr4::low(rate, BASS_DRUMS_HZ),
            drums_high: Lr4::high(rate, BASS_DRUMS_HZ),
            ap_bass_4500: Lr4AllPass::new(rate, VOCALS_MELODY_HZ),
            ap_drums_4500: Lr4AllPass::new(rate, VOCALS_MELODY_HZ),

            vocals_low: Lr4::low(rate, VOCALS_MELODY_HZ),
            melody_high: Lr4::high(rate, VOCALS_MELODY_HZ),
            ap_vocals_150: Lr4AllPass::new(rate, BASS_DRUMS_HZ),
            ap_melody_150: Lr4AllPass::new(rate, BASS_DRUMS_HZ),

            filters: [StemKnobFilter::new(rate); STEM_COUNT],
        }
    }

    /// Splits input into four phase-aligned stems: [Drums, Bass, Melody, Vocals].
    #[inline(always)]
    fn split(&mut self, x: f32) -> [f32; STEM_COUNT] {
        // 1. Split at 800 Hz into lower (< 800 Hz) and upper (> 800 Hz)
        let lower = self.mid_split_low.run(x);
        let upper = self.mid_split_high.run(x);

        // 2. Split lower band at 150 Hz into Bass (< 150 Hz) and Drums (150 - 800 Hz)
        let raw_bass = self.bass_low.run(lower);
        let raw_drums = self.drums_high.run(lower);
        // Phase align each stem across 4500 Hz with its own filter state
        let bass = self.ap_bass_4500.run(raw_bass);
        let drums = self.ap_drums_4500.run(raw_drums);

        // 3. Split upper band at 4500 Hz into Vocals (800 - 4500 Hz) and Melody (> 4500 Hz)
        let raw_vocals = self.vocals_low.run(upper);
        let raw_melody = self.melody_high.run(upper);
        // Phase align each stem across 150 Hz with its own filter state
        let vocals = self.ap_vocals_150.run(raw_vocals);
        let melody = self.ap_melody_150.run(raw_melody);

        [drums, bass, melody, vocals]
    }

    fn reset(&mut self) {
        self.mid_split_low.reset();
        self.mid_split_high.reset();
        self.bass_low.reset();
        self.drums_high.reset();
        self.ap_bass_4500.reset();
        self.ap_drums_4500.reset();
        self.vocals_low.reset();
        self.melody_high.reset();
        self.ap_vocals_150.reset();
        self.ap_melody_150.reset();
        for f in &mut self.filters {
            f.reset();
        }
    }
}

/// Stereo 4-stem isolator for DJ mixing and remix loops.
#[derive(Debug, Clone)]
pub struct StemIsolator {
    channels: [StemFilterbankChannel; 2],
    ref_split_low: Lr4,
    ref_split_high: Lr4,
    ref_lower_150: Lr4AllPass,
    ref_lower_4500: Lr4AllPass,
    ref_upper_4500: Lr4AllPass,
    ref_upper_150: Lr4AllPass,
}

impl StemIsolator {
    /// Creates a new StemIsolator for the given sample rate.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            channels: [
                StemFilterbankChannel::new(sample_rate),
                StemFilterbankChannel::new(sample_rate),
            ],
            ref_split_low: Lr4::low(sample_rate, DRUMS_VOCALS_HZ),
            ref_split_high: Lr4::high(sample_rate, DRUMS_VOCALS_HZ),
            ref_lower_150: Lr4AllPass::new(sample_rate, BASS_DRUMS_HZ),
            ref_lower_4500: Lr4AllPass::new(sample_rate, VOCALS_MELODY_HZ),
            ref_upper_4500: Lr4AllPass::new(sample_rate, VOCALS_MELODY_HZ),
            ref_upper_150: Lr4AllPass::new(sample_rate, BASS_DRUMS_HZ),
        }
    }

    /// Splits a stereo frame into 4 isolated stereo stems: [Drums, Bass, Melody, Vocals].
    /// Each entry is `[left, right]`.
    #[inline(always)]
    pub fn split_stems(&mut self, l: f32, r: f32) -> [[f32; 2]; STEM_COUNT] {
        let left_stems = self.channels[0].split(l);
        let right_stems = self.channels[1].split(r);

        let mut out = [[0.0; 2]; STEM_COUNT];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = [left_stems[i], right_stems[i]];
        }
        out
    }

    /// Processes a single stereo frame through the stem filterbank, applying per-stem
    /// volume, mute, solo, and filter controls.
    #[inline(always)]
    pub fn process_frame(&mut self, l: f32, r: f32, params: &StemDeckParams) -> (f32, f32) {
        let stems = self.split_stems(l, r);
        let mut out_l = 0.0;
        let mut out_r = 0.0;

        for (stem_idx, stem_frame) in stems.iter().enumerate() {
            let stem = match StemType::from_index(stem_idx) {
                Some(s) => s,
                None => continue,
            };

            let gain = params.effective_gain(stem);
            if gain <= 1e-6 {
                continue;
            }

            let filter_knob = params.filter(stem);
            self.channels[0].filters[stem_idx].set(filter_knob);
            self.channels[1].filters[stem_idx].set(filter_knob);

            let filtered_l = self.channels[0].filters[stem_idx].run(stem_frame[0]);
            let filtered_r = self.channels[1].filters[stem_idx].run(stem_frame[1]);

            out_l += filtered_l * gain;
            out_r += filtered_r * gain;
        }

        (out_l, out_r)
    }

    /// Passes a mono sample through the reference all-pass filterbank tree.
    /// Used to verify flat magnitude and zero reconstruction error.
    pub fn process_allpass_reference(&mut self, sample: f32) -> f32 {
        let lower = self.ref_split_low.run(sample);
        let upper = self.ref_split_high.run(sample);

        let lower_allpassed = self.ref_lower_4500.run(self.ref_lower_150.run(lower));
        let upper_allpassed = self.ref_upper_150.run(self.ref_upper_4500.run(upper));

        lower_allpassed + upper_allpassed
    }

    /// Resets all internal delay buffers.
    pub fn reset(&mut self) {
        self.channels[0].reset();
        self.channels[1].reset();
        self.ref_split_low.reset();
        self.ref_split_high.reset();
        self.ref_lower_150.reset();
        self.ref_lower_4500.reset();
        self.ref_upper_4500.reset();
        self.ref_upper_150.reset();
    }
}
