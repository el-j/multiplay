//! Tempo, beat phase and downbeat from an onset envelope.
//!
//! 1. Coarse tempo: the autocorrelation peak of the onset envelope between 60 and 200 BPM, moved
//!    to the faster octave when that one is nearly as strong and still within the DJ range.
//! 2. Fine tempo and phase: the envelope is folded modulo each candidate beat period; the period
//!    whose folded onsets pile up most sharply wins. Over a whole track a period error of a few
//!    hundredths of a BPM smears the pile, so this resolves tempo far below one envelope step.
//! 3. Downbeat: the beat (of four) with the most low-band onset energy starts the bar.

#![forbid(unsafe_code)]

const MIN_BPM: f64 = 60.0;
const MAX_BPM: f64 = 200.0;
/// Tempos are reported within this range (the usual DJ range).
const RANGE: (f64, f64) = (85.0, 176.0);
const BINS: usize = 64;

/// Onset envelopes sampled every `hop_secs` seconds.
pub(crate) struct Envelope<'a> {
    /// Full-band onset strength.
    pub full: &'a [f32],
    /// Low-band (kick) onset strength.
    pub low: &'a [f32],
    pub hop_secs: f64,
}

/// Tempo in BPM and the time of a downbeat, or `None` without a steady beat.
pub(crate) fn beat_grid(env: &Envelope<'_>) -> Option<(f64, f64)> {
    let total: f32 = env.full.iter().sum();
    if env.full.len() < 64 || total < 1e-3 * env.full.len() as f32 {
        return None;
    }
    let coarse = coarse_bpm(env.full, env.hop_secs)?;
    let mut best = (coarse, f64::MIN, 0.0);
    for (span, step) in [(0.03, 0.05), (0.0006, 0.002)] {
        let center = best.0;
        let lo = center * (1.0 - span);
        let n = ((center * 2.0 * span) / step).ceil() as usize;
        for k in 0..=n {
            let bpm = lo + k as f64 * step;
            let (score, phase) = fold(env.full, env.hop_secs, 60.0 / bpm);
            if score > best.1 {
                best = (bpm, score, phase);
            }
        }
    }
    let (bpm, _, phase) = best;
    let bpm = (bpm * 1_000.0).round() / 1_000.0;
    Some((bpm, downbeat(env, 60.0 / bpm, phase)))
}

fn coarse_bpm(o: &[f32], hop: f64) -> Option<f64> {
    let mean = o.iter().sum::<f32>() / o.len() as f32;
    let x: Vec<f32> = o.iter().map(|v| v - mean).collect();
    let lag_of = |bpm: f64| (60.0 / bpm / hop).round() as usize;
    let (min_lag, max_lag) = (lag_of(MAX_BPM).max(1), lag_of(MIN_BPM).min(x.len() / 2));
    let acf = |lag: usize| -> f32 {
        let n = x.len() - lag;
        x[..n]
            .iter()
            .zip(&x[lag..])
            .map(|(a, b)| a * b)
            .sum::<f32>()
            / n as f32
    };
    let r: Vec<f32> = (min_lag..=max_lag).map(acf).collect();
    let (i, &peak) = r.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    if peak <= 0.0 {
        return None;
    }
    let lag = min_lag + i;
    let mut bpm = 60.0 / (lag as f64 * hop);
    // Prefer the faster octave when it is nearly as strong (a track at 87 with clicks on every
    // beat must stay 87, but a 174 track whose bars dominate must not read as 87).
    let half = (lag as f64 / 2.0).round() as usize;
    if half >= min_lag && 2.0 * bpm < RANGE.1 && r[half - min_lag] >= 0.5 * peak {
        bpm *= 2.0;
    }
    while bpm < RANGE.0 {
        bpm *= 2.0;
    }
    while bpm >= RANGE.1 {
        bpm /= 2.0;
    }
    Some(bpm)
}

/// Folds the envelope modulo `period` seconds. Returns how sharply onsets pile up in one place
/// (peak of three neighbouring bins over the mean) and where that place is, in seconds.
fn fold(o: &[f32], hop: f64, period: f64) -> (f64, f64) {
    let mut bins = [0.0f64; BINS];
    for (i, &v) in o.iter().enumerate() {
        if v > 0.0 {
            let pos = (i as f64 * hop / period).fract();
            bins[(pos * BINS as f64) as usize % BINS] += f64::from(v);
        }
    }
    let at = |k: isize| bins[k.rem_euclid(BINS as isize) as usize];
    let (mut best, mut best_k) = (f64::MIN, 0isize);
    for k in 0..BINS as isize {
        let s = at(k - 1) + at(k) + at(k + 1);
        if s > best {
            best = s;
            best_k = k;
        }
    }
    let mean = bins.iter().sum::<f64>() / BINS as f64;
    // Phase: weighted centre of the five bins around the pile, in bin units.
    let (mut w, mut c) = (0.0, 0.0);
    for d in -2..=2isize {
        let v = at(best_k + d);
        w += v;
        c += v * (best_k + d) as f64;
    }
    let centre = if w > 0.0 { c / w } else { best_k as f64 };
    let phase = (centre / BINS as f64).rem_euclid(1.0) * period;
    (best / (3.0 * mean.max(1e-12)), phase)
}

/// Picks the beat of four that carries the most low-band onset energy and returns the earliest
/// such downbeat at or after 0 s.
fn downbeat(env: &Envelope<'_>, period: f64, phase: f64) -> f64 {
    let mut sums = [0.0f64; 4];
    let mut n = 0usize;
    loop {
        let t = phase + n as f64 * period;
        let i = (t / env.hop_secs).round() as usize;
        if i >= env.low.len() {
            break;
        }
        // A kick's low-band energy keeps rising for a few ms after the beat (filter delay, slow
        // low frequencies), so sum the onset over a short window that follows the beat.
        let lo = i.saturating_sub(2);
        let hi = (i + 11).min(env.low.len());
        let energy: f32 = env.low[lo..hi].iter().sum();
        sums[n % 4] += f64::from(energy);
        n += 1;
    }
    let g = (0..4)
        .max_by(|a, b| sums[*a].total_cmp(&sums[*b]))
        .unwrap_or(0);
    (phase + g as f64 * period).rem_euclid(4.0 * period)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pulses(bpm: f64, hop: f64, secs: f64, offset: f64) -> Vec<f32> {
        let n = (secs / hop) as usize;
        let mut o = vec![0.0f32; n];
        let mut k = 0;
        loop {
            let i = ((offset + k as f64 * 60.0 / bpm) / hop).round() as usize;
            if i >= n {
                break;
            }
            o[i] = 1.0;
            k += 1;
        }
        o
    }

    #[test]
    fn dj_analysis_coarse_tempo_keeps_slow_tracks_slow_and_fast_tracks_fast() {
        let hop = 128.0 / 44_100.0;
        for bpm in [87.0, 120.0, 174.0] {
            let got = coarse_bpm(&pulses(bpm, hop, 60.0, 0.1), hop).unwrap();
            assert!((got - bpm).abs() / bpm < 0.03, "{bpm} read as {got}");
        }
    }

    #[test]
    fn dj_analysis_fold_finds_the_phase_of_a_pulse_train() {
        let hop = 0.001;
        let (score, phase) = fold(&pulses(120.0, hop, 30.0, 0.2), hop, 0.5);
        assert!(score > 10.0);
        assert!((phase - 0.2).abs() < 0.01, "{phase}");
    }
}
