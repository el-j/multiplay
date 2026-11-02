//! Remix loop slots: capture beat-synced loops from any deck into a slot grid (TASK-221).
//!
//! Real-time audio safety:
//! Zero heap allocations, zero system calls, and zero locks on the audio render thread.

#![forbid(unsafe_code)]

use super::engine::TrackBuffer;
use crate::dsp::BiquadFilter;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Number of remix slots in the grid.
pub const NUM_SLOTS: usize = 8;
const Q: f32 = std::f32::consts::FRAC_1_SQRT_2;
const FILTER_DEAD_ZONE: f32 = 0.02;

/// A captured loop segment stored in memory.
#[derive(Debug, Clone)]
pub struct CapturedLoop {
    sample_rate: u32,
    bpm: f64,
    anchor: f64,
    samples: Arc<Vec<f32>>, // Interleaved stereo samples
}

impl CapturedLoop {
    /// Captures a loop from a [`TrackBuffer`] between `start_secs` and `end_secs`.
    pub fn capture(
        buffer: &TrackBuffer,
        start_secs: f64,
        end_secs: f64,
        bpm: f64,
        anchor: f64,
    ) -> Option<Self> {
        let sample_rate = buffer.sample_rate();
        if sample_rate == 0 || end_secs <= start_secs {
            return None;
        }

        let rate_f64 = f64::from(sample_rate);
        let start_frame = (start_secs * rate_f64).round() as usize;
        let end_frame = (end_secs * rate_f64).round() as usize;
        if end_frame <= start_frame {
            return None;
        }

        let frame_count = end_frame - start_frame;
        let mut samples = Vec::with_capacity(frame_count * 2);

        for i in start_frame..end_frame {
            let (l, r) = buffer.frame(i)?;
            samples.push(l);
            samples.push(r);
        }

        Some(Self {
            sample_rate,
            bpm,
            anchor,
            samples: Arc::new(samples),
        })
    }

    /// Creates a captured loop directly from an interleaved stereo sample slice.
    pub fn from_samples(sample_rate: u32, bpm: f64, anchor: f64, samples: &[f32]) -> Self {
        Self {
            sample_rate,
            bpm,
            anchor,
            samples: Arc::new(samples.to_vec()),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn anchor(&self) -> f64 {
        self.anchor
    }

    pub fn length_frames(&self) -> usize {
        self.samples.len() / 2
    }

    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.length_frames() as f64 / f64::from(self.sample_rate)
        }
    }

    #[inline]
    pub fn frame(&self, i: usize) -> (f32, f32) {
        let total = self.length_frames();
        if total == 0 {
            return (0.0, 0.0);
        }
        let idx = (i % total) * 2;
        (self.samples[idx], self.samples[idx + 1])
    }

    /// Separates this captured loop into four stems (Drums, Bass, Melody, Vocals).
    #[must_use]
    pub fn separate_stems(&self) -> super::stems::SeparatedStems {
        super::stems::SeparatedStems::separate(&self.samples, self.sample_rate as f32)
    }
}

/// Metadata and settings of a single remix slot for session persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemixSlotConfig {
    pub index: usize,
    pub volume: f32,
    pub filter: f32,
    pub muted: bool,
    pub bpm: f64,
}

/// Atomic control parameters for one remix slot.
#[derive(Debug)]
pub struct RemixSlotParams {
    volume: AtomicU32,
    filter: AtomicU32,
    playing: AtomicBool,
    muted: AtomicBool,
}

impl Default for RemixSlotParams {
    fn default() -> Self {
        Self {
            volume: AtomicU32::new(1.0f32.to_bits()),
            filter: AtomicU32::new(0.0f32.to_bits()),
            playing: AtomicBool::new(false),
            muted: AtomicBool::new(false),
        }
    }
}

impl RemixSlotParams {
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    pub fn set_volume(&self, v: f32) {
        self.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn filter(&self) -> f32 {
        f32::from_bits(self.filter.load(Ordering::Relaxed))
    }

    pub fn set_filter(&self, f: f32) {
        self.filter.store(f.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn set_playing(&self, p: bool) {
        self.playing.store(p, Ordering::Relaxed);
    }

    pub fn muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    pub fn set_muted(&self, m: bool) {
        self.muted.store(m, Ordering::Relaxed);
    }
}

/// Group of params for the entire remix grid.
#[derive(Debug, Default)]
pub struct RemixParams {
    slots: [RemixSlotParams; NUM_SLOTS],
}

impl RemixParams {
    pub fn slot(&self, idx: usize) -> &RemixSlotParams {
        &self.slots[idx.min(NUM_SLOTS - 1)]
    }
}

/// Player instance for a single remix slot on the audio thread.
#[derive(Debug)]
struct SlotPlayer {
    loop_data: Option<CapturedLoop>,
    filter_lp: [BiquadFilter; 2],
    filter_hp: [BiquadFilter; 2],
    filter_knob: f32,
}

impl SlotPlayer {
    fn new(sample_rate: f32) -> Self {
        Self {
            loop_data: None,
            filter_lp: [BiquadFilter::new_lowpass(sample_rate, 20_000.0, Q); 2],
            filter_hp: [BiquadFilter::new_highpass(sample_rate, 20.0, Q); 2],
            filter_knob: 0.0,
        }
    }

    fn update_filter(&mut self, rate: f32, knob: f32) {
        if (knob - self.filter_knob).abs() < 1e-4 {
            return;
        }
        self.filter_knob = knob;
        let k = knob.clamp(-1.0, 1.0);
        if k < -FILTER_DEAD_ZONE {
            let cutoff = 20_000.0 * (60.0f32 / 20_000.0).powf(-k);
            let target = BiquadFilter::new_lowpass(rate, cutoff, Q);
            self.filter_lp[0].retune(&target);
            self.filter_lp[1].retune(&target);
        } else if k > FILTER_DEAD_ZONE {
            let cutoff = 20.0 * (8_000.0f32 / 20.0).powf(k);
            let target = BiquadFilter::new_highpass(rate, cutoff, Q);
            self.filter_hp[0].retune(&target);
            self.filter_hp[1].retune(&target);
        }
    }

    #[inline]
    fn process_filter(&mut self, mut l: f32, mut r: f32) -> (f32, f32) {
        let k = self.filter_knob;
        if k < -FILTER_DEAD_ZONE {
            l = self.filter_lp[0].process_sample(l);
            r = self.filter_lp[1].process_sample(r);
        } else if k > FILTER_DEAD_ZONE {
            l = self.filter_hp[0].process_sample(l);
            r = self.filter_hp[1].process_sample(r);
        }
        (l, r)
    }
}

/// The remix slot grid audio processor.
#[derive(Debug)]
pub struct RemixGrid {
    sample_rate: u32,
    params: Arc<RemixParams>,
    slots: [SlotPlayer; NUM_SLOTS],
}

impl RemixGrid {
    pub fn new(sample_rate: u32) -> Self {
        let rate_f32 = sample_rate as f32;
        Self {
            sample_rate,
            params: Arc::new(RemixParams::default()),
            slots: std::array::from_fn(|_| SlotPlayer::new(rate_f32)),
        }
    }

    pub fn params(&self) -> Arc<RemixParams> {
        Arc::clone(&self.params)
    }

    pub fn load_slot(&mut self, slot_idx: usize, loop_data: Option<CapturedLoop>) {
        if slot_idx < NUM_SLOTS {
            self.slots[slot_idx].loop_data = loop_data;
        }
    }

    /// Renders all active remix slots into `out` (interleaved stereo).
    /// Audio is beat-quantized and phase-locked to `master_bpm` and `master_position_secs`.
    pub fn render(&mut self, out: &mut [f32], master_bpm: f64, master_position_secs: f64) {
        out.fill(0.0);
        if self.sample_rate == 0 || master_bpm <= 0.0 {
            return;
        }

        let rate_f64 = f64::from(self.sample_rate);
        let rate_f32 = self.sample_rate as f32;
        let num_frames = out.len() / 2;

        for (idx, slot) in self.slots.iter_mut().enumerate() {
            let p = self.params.slot(idx);
            if !p.playing() || p.muted() {
                continue;
            }
            let vol = p.volume();
            if vol <= 0.0 {
                continue;
            }

            let Some(loop_data) = slot.loop_data.clone() else {
                continue;
            };
            let loop_frames = loop_data.length_frames();
            if loop_frames == 0 {
                continue;
            }

            slot.update_filter(rate_f32, p.filter());

            // Speed ratio: match heard master tempo
            let ratio = master_bpm / loop_data.bpm();
            let step = (f64::from(loop_data.sample_rate()) / rate_f64) * ratio;

            // Phase lock: align with master beat grid
            let master_beats = master_position_secs * master_bpm / 60.0;
            let loop_beats = (loop_frames as f64 / f64::from(loop_data.sample_rate())) * loop_data.bpm() / 60.0;
            let current_loop_beat = if loop_beats > 0.0 {
                master_beats.rem_euclid(loop_beats)
            } else {
                0.0
            };

            let mut pos = (current_loop_beat / (loop_data.bpm() / 60.0)) * f64::from(loop_data.sample_rate());

            for f in 0..num_frames {
                let (raw_l, raw_r) = read_loop_frame(&loop_data, pos);
                let (filt_l, filt_r) = slot.process_filter(raw_l, raw_r);
                out[f * 2] += filt_l * vol;
                out[f * 2 + 1] += filt_r * vol;
                pos += step;
                if pos >= loop_frames as f64 {
                    pos %= loop_frames as f64;
                }
            }
        }
    }
}

/// 4-point Hermite interpolation across loop boundaries.
#[inline]
fn read_loop_frame(loop_data: &CapturedLoop, pos: f64) -> (f32, f32) {
    let total = loop_data.length_frames();
    if total == 0 {
        return (0.0, 0.0);
    }
    let p = pos.rem_euclid(total as f64);
    let i = p.floor() as usize;
    let t = (p - i as f64) as f32;

    let at = |offset: isize| {
        let idx = (i as isize + offset).rem_euclid(total as isize) as usize;
        loop_data.frame(idx)
    };

    let (a, b, c, d) = (at(-1), at(0), at(1), at(2));
    (
        hermite(a.0, b.0, c.0, d.0, t),
        hermite(a.1, b.1, c.1, d.1, t),
    )
}

#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}
