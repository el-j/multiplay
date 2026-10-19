//! Offline DJ track analysis (TASK-204): BPM, beat grid with downbeat, a whole-track overview
//! and a three-band detail waveform. Runs on a background thread over decoded audio, never on
//! the audio thread; results are cached through [`amp_ports::AnalysisCachePort`].

#![forbid(unsafe_code)]

mod cache;
mod loudness;
mod tempo;

pub use cache::spawn_analysis;

use crate::dj::engine::TrackBuffer;
use crate::dsp::BiquadFilter;
use amp_domain::{TrackAnalysis, ANALYSIS_VERSION};

/// Overview columns for the whole track.
const OVERVIEW_COLUMNS: usize = 1_024;
/// Detail waveform columns per second.
const DETAIL_RATE: f32 = 100.0;
/// Onset envelope steps per second (about 2.9 ms).
const ENVELOPE_RATE: u32 = 344;

/// Analyses interleaved stereo `samples` at `rate`.
pub fn analyze(rate: u32, samples: &[f32]) -> TrackAnalysis {
    let frames = samples.len() / 2;
    let mut a = Analyzer::new(rate, frames);
    for f in samples.as_chunks::<2>().0 {
        a.push(f[0], f[1]);
    }
    a.finish()
}

/// Analyses a deck's track once it is fully decoded; `None` while it is still loading.
pub fn analyze_track_buffer(buf: &TrackBuffer) -> Option<TrackAnalysis> {
    if !buf.is_complete() || buf.sample_rate() == 0 {
        return None;
    }
    let frames = buf.loaded_frames();
    let mut a = Analyzer::new(buf.sample_rate(), frames);
    for i in 0..frames {
        let (l, r) = buf.frame(i)?;
        a.push(l, r);
    }
    Some(a.finish())
}

/// Streaming analyser: one pass over the audio feeds the onset envelopes and both waveforms.
struct Analyzer {
    rate: u32,
    frames: usize,
    seen: usize,
    hop: usize,
    kick: BiquadFilter,
    hop_full: f32,
    hop_low: f32,
    energy_full: Vec<f32>,
    energy_low: Vec<f32>,
    overview: Vec<f32>,
    column: usize,
    bands: [BiquadFilter; 4],
    band_sums: [f32; 3],
    detail: Vec<[u8; 3]>,
    loudness: loudness::Meter,
}

impl Analyzer {
    fn new(rate: u32, frames: usize) -> Self {
        let r = rate.max(1) as f32;
        Self {
            rate: rate.max(1),
            frames,
            seen: 0,
            hop: (rate / ENVELOPE_RATE).max(1) as usize,
            kick: BiquadFilter::new_lowpass(r, 150.0, 0.707),
            hop_full: 0.0,
            hop_low: 0.0,
            energy_full: Vec::with_capacity(frames / 128 + 1),
            energy_low: Vec::with_capacity(frames / 128 + 1),
            overview: vec![0.0; OVERVIEW_COLUMNS],
            column: ((r / DETAIL_RATE) as usize).max(1),
            bands: [
                BiquadFilter::new_lowpass(r, 200.0, 0.707),
                BiquadFilter::new_highpass(r, 200.0, 0.707),
                BiquadFilter::new_lowpass(r, 2_500.0, 0.707),
                BiquadFilter::new_highpass(r, 2_500.0, 0.707),
            ],
            band_sums: [0.0; 3],
            detail: Vec::with_capacity(frames / 400 + 1),
            loudness: loudness::Meter::new(rate.max(1)),
        }
    }

    fn push(&mut self, l: f32, r: f32) {
        self.loudness.push(l, r);
        let m = 0.5 * (l + r);
        let k = self.kick.process_sample(m);
        self.hop_full += m * m;
        self.hop_low += k * k;

        let col = self.seen * OVERVIEW_COLUMNS / self.frames.max(1);
        if let Some(v) = self.overview.get_mut(col) {
            *v = v.max(l.abs().max(r.abs()));
        }

        let low = self.bands[0].process_sample(m);
        let above_low = self.bands[1].process_sample(m);
        let mid = self.bands[2].process_sample(above_low);
        let high = self.bands[3].process_sample(m);
        for (s, v) in self.band_sums.iter_mut().zip([low, mid, high]) {
            *s += v * v;
        }

        self.seen += 1;
        if self.seen.is_multiple_of(self.hop) {
            let n = self.hop as f32;
            self.energy_full
                .push((1.0 + 1_000.0 * self.hop_full / n).ln());
            self.energy_low
                .push((1.0 + 1_000.0 * self.hop_low / n).ln());
            self.hop_full = 0.0;
            self.hop_low = 0.0;
        }
        if self.seen.is_multiple_of(self.column) {
            let n = self.column as f32;
            let level = |s: f32| ((s / n).sqrt() * 2.0 * 255.0).min(255.0) as u8;
            self.detail.push(self.band_sums.map(level));
            self.band_sums = [0.0; 3];
        }
    }

    fn finish(self) -> TrackAnalysis {
        let full = flux(&self.energy_full);
        let low = flux(&self.energy_low);
        let env = tempo::Envelope {
            full: &full,
            low: &low,
            hop_secs: self.hop as f64 / f64::from(self.rate),
        };
        let grid = tempo::beat_grid(&env);
        TrackAnalysis {
            version: ANALYSIS_VERSION,
            duration_secs: self.frames as f64 / f64::from(self.rate),
            bpm: grid.map(|g| g.0),
            first_downbeat_secs: grid.map_or(0.0, |g| g.1),
            beats_per_bar: 4,
            overview: self
                .overview
                .iter()
                .map(|v| (v.min(1.0) * 255.0) as u8)
                .collect(),
            detail: self.detail,
            detail_rate: self.rate as f32 / self.column as f32,
            loudness_lufs: self.loudness.integrated(),
        }
    }
}

/// Positive change of a log-energy curve: how strongly something starts at each step.
fn flux(e: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(e.len());
    let mut prev = e.first().copied().unwrap_or(0.0);
    for &v in e {
        out.push((v - prev).max(0.0));
        prev = v;
    }
    out
}
