//! Track analysis (TASK-204): tempo within ±0.1 BPM and beat phase within 10 ms on click
//! tracks across the DJ range, downbeats from accents, waveforms, and speed.

use amp_core_engine::dj::analysis::{analyze, analyze_track_buffer};
use amp_core_engine::dj::engine::TrackBuffer;
use amp_domain::ANALYSIS_VERSION;

/// Interleaved stereo click track: a short 1 kHz click with a low thump on every beat, the first
/// beat of each bar twice as loud, starting at `offset` seconds, over quiet noise.
fn click_track(rate: u32, bpm: f64, offset: f64, secs: f64) -> Vec<f32> {
    let frames = (f64::from(rate) * secs) as usize;
    let mut out = vec![0.0f32; frames * 2];
    let mut seed = 12_345u32;
    for f in 0..frames {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((seed >> 9) as f32 / (1u32 << 23) as f32 - 0.5) * 0.01;
        out[f * 2] = noise;
        out[f * 2 + 1] = noise;
    }
    let period = 60.0 / bpm;
    let mut n = 0usize;
    loop {
        let t = offset + n as f64 * period;
        let start = (t * f64::from(rate)).round() as usize;
        if start >= frames {
            break;
        }
        let amp = if n.is_multiple_of(4) { 0.8 } else { 0.4 };
        for k in 0..(rate as usize / 50) {
            let Some(i) = (start + k).checked_mul(2).filter(|i| *i + 1 < out.len()) else {
                break;
            };
            let tt = k as f32 / rate as f32;
            let env = (-tt * 120.0).exp();
            let v = amp
                * env
                * ((2.0 * std::f32::consts::PI * 1_000.0 * tt).sin() * 0.5
                    + (2.0 * std::f32::consts::PI * 60.0 * tt).sin());
            out[i] += v;
            out[i + 1] += v;
        }
        n += 1;
    }
    out
}

/// Distance from `t` to the nearest beat of an ideal grid, in seconds.
fn phase_error(bpm: f64, grid_point: f64, t_true: f64) -> f64 {
    let p = 60.0 / bpm;
    let d = (grid_point - t_true).rem_euclid(p);
    d.min(p - d)
}

#[test]
fn dj_analysis_finds_tempo_and_phase_across_the_dj_range() {
    for (bpm, offset) in [
        (90.0, 0.31),
        (100.0, 0.05),
        (120.0, 0.437),
        (126.0, 0.2),
        (128.0, 0.0),
        (140.0, 0.123),
        (150.0, 0.333),
        (174.0, 0.07),
    ] {
        let samples = click_track(44_100, bpm, offset, 60.0);
        let a = analyze(44_100, &samples);
        let got = a.bpm.unwrap_or(0.0);
        assert!((got - bpm).abs() <= 0.1, "{bpm} BPM detected as {got}");
        let err = phase_error(bpm, a.first_downbeat_secs, offset);
        assert!(
            err < 0.010,
            "{bpm} BPM: beat phase off by {:.1} ms",
            err * 1000.0
        );
    }
}

#[test]
fn dj_analysis_handles_fractional_tempos() {
    let samples = click_track(48_000, 123.45, 0.2, 90.0);
    let a = analyze(48_000, &samples);
    assert!((a.bpm.unwrap() - 123.45).abs() <= 0.05, "{:?}", a.bpm);
}

#[test]
fn dj_analysis_puts_the_downbeat_on_the_accented_beat() {
    for (bpm, offset) in [(96.0, 0.9), (128.0, 0.25), (140.0, 1.3), (172.0, 0.05)] {
        let samples = click_track(44_100, bpm, offset, 45.0);
        let a = analyze(44_100, &samples);
        // Accented beats are 0, 4, 8, ... after the offset: the anchor must be one of them.
        let bar = 4.0 * 60.0 / bpm;
        let d = (a.first_downbeat_secs - offset).rem_euclid(bar);
        let err = d.min(bar - d);
        assert!(
            err < 0.010,
            "{bpm} BPM: downbeat {:.3}s is not on a bar start",
            a.first_downbeat_secs
        );
        assert_eq!(a.beats_per_bar, 4);
    }
}

#[test]
fn dj_analysis_reports_no_tempo_for_silence() {
    let a = analyze(44_100, &vec![0.0; 44_100 * 2 * 10]);
    assert_eq!(a.bpm, None);
}

#[test]
fn dj_analysis_builds_overview_and_three_band_waveforms() {
    let rate = 44_100;
    let samples = click_track(rate, 120.0, 0.0, 20.0);
    let a = analyze(rate, &samples);
    assert_eq!(a.version, ANALYSIS_VERSION);
    assert!((a.duration_secs - 20.0).abs() < 0.01);
    assert_eq!(a.overview.len(), 1_024);
    assert!(
        a.overview.iter().any(|&v| v > 100),
        "clicks show up in the overview"
    );
    let expected = (20.0 * a.detail_rate) as usize;
    assert!((a.detail.len() as i64 - expected as i64).abs() <= 1);
    let low: u32 = a.detail.iter().map(|c| u32::from(c[0])).sum();
    let high: u32 = a.detail.iter().map(|c| u32::from(c[2])).sum();
    assert!(
        low > 0 && high > 0,
        "thump in the low band, click in the high band"
    );
}

#[test]
fn dj_analysis_reads_a_deck_track_buffer() {
    let samples = click_track(44_100, 124.0, 0.1, 30.0);
    let buf = TrackBuffer::from_samples(44_100, &samples);
    let a = analyze_track_buffer(&buf).expect("complete buffer");
    assert!((a.bpm.unwrap() - 124.0).abs() <= 0.1);
}

/// Release-build speed target: a 5-minute track in under 3 s. Run with
/// `cargo test --release -p amp-core-engine dj_analysis_is_fast`.
#[test]
#[cfg_attr(debug_assertions, ignore = "timing target applies to release builds")]
fn dj_analysis_is_fast() {
    let samples = click_track(44_100, 128.0, 0.0, 300.0);
    let start = std::time::Instant::now();
    let a = analyze(44_100, &samples);
    let took = start.elapsed();
    assert!(a.bpm.is_some());
    assert!(took.as_secs_f64() < 3.0, "5-minute track took {took:?}");
}

fn sine(rate: u32, hz: f64, amp: f64, secs: f64) -> Vec<f32> {
    (0..(f64::from(rate) * secs) as usize)
        .flat_map(|i| {
            let v =
                (amp * (2.0 * std::f64::consts::PI * hz * i as f64 / f64::from(rate)).sin()) as f32;
            [v, v]
        })
        .collect()
}

#[test]
fn dj_analysis_measures_integrated_loudness() {
    // EBU R128 reference: a 1 kHz stereo sine at -20 dBFS reads -20 LUFS.
    for rate in [44_100, 48_000] {
        let a = analyze(rate, &sine(rate, 1_000.0, 0.1, 20.0));
        let lufs = a.loudness_lufs.expect("loud enough to gate");
        assert!((lufs + 20.0).abs() < 0.05, "{rate} Hz: {lufs:.2} LUFS");
    }
    let quiet = analyze(48_000, &sine(48_000, 1_000.0, 0.01, 20.0));
    assert!((quiet.loudness_lufs.unwrap() + 40.0).abs() < 0.05);
    assert_eq!(
        analyze(48_000, &vec![0.0; 96_000]).loudness_lufs,
        None,
        "silence is gated out"
    );
}

#[test]
fn dj_analysis_loudness_ignores_silent_parts() {
    // Ten seconds of tone and ten of near-silence read like the tone alone (relative gate).
    let mut s = sine(48_000, 1_000.0, 0.1, 10.0);
    s.extend(sine(48_000, 1_000.0, 0.0001, 10.0));
    let lufs = analyze(48_000, &s).loudness_lufs.unwrap();
    // Blocks straddling the change pass the relative gate and pull it down slightly.
    assert!((lufs + 20.07).abs() < 0.05, "{lufs:.2}");
}
