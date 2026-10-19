//! DJ runtime against wall-clock time (TASK-202/203): it keeps a simulated sound card fed and
//! routes the cue bus. These tests live in their own binary so no CPU-heavy test runs beside
//! them; cargo runs test binaries one after another.

use amp_core_engine::dj::engine::{
    CueRouting, DjEngine, DjOutputs, DjRuntime, TrackBuffer, CROSSFADER_LEFT,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn sine(rate: u32, hz: f32, secs: f32) -> Arc<TrackBuffer> {
    let frames = (rate as f32 * secs) as usize;
    let mut s = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let v = 0.5 * (2.0 * std::f32::consts::PI * hz * i as f32 / rate as f32).sin();
        s.extend([v, v]);
    }
    TrackBuffer::from_samples(rate, &s)
}

#[test]
fn dj_runtime_keeps_a_real_time_output_fed() {
    let (engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, sine(44_100, 440.0, 10.0));
    control.params().deck(0).set_playing(true);
    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let lead = 4_800; // 100 ms: margin for a busy CI machine running a debug build
    let out = DjOutputs {
        main: Box::new(producer),
        main_channels: 2,
        phones: None,
    };
    let runtime = DjRuntime::start(engine, out, lead);

    // A fake sound card: takes 10 ms of audio every 10 ms of wall time.
    std::thread::sleep(Duration::from_millis(100));
    let starved = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let mut taken = 0usize;
    while start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
        let due = (start.elapsed().as_secs_f64() * 48_000.0) as usize * 2;
        let want = due.saturating_sub(taken);
        let got = consumer.read_chunk(want.min(consumer.slots())).map(|c| {
            let n = c.len();
            c.commit_all();
            n
        });
        let got = got.unwrap_or(0);
        if got < want {
            starved.store(true, Ordering::Relaxed);
        }
        taken += got;
        assert!(
            consumer.slots() <= lead * 2 + 2_048,
            "latency stays near the lead: {} samples queued",
            consumer.slots()
        );
    }
    runtime.stop();
    assert!(!starved.load(Ordering::Relaxed), "output starved");
    assert_eq!(control.status().deck(0).underruns(), 0);
}

#[test]
fn dj_cue_split_mode_prelistens_on_the_right_while_master_plays_left() {
    let (engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, sine(48_000, 300.0, 5.0));
    control.load(1, sine(48_000, 900.0, 5.0));
    let p = control.params();
    p.set_crossfader(CROSSFADER_LEFT);
    p.deck(0).set_playing(true);
    p.deck(1).set_fader(0.0); // silent on master...
    p.deck(1).set_pfl(true); // ...but in the headphones
    p.deck(1).set_playing(true);
    p.set_cue_volume(1.0);
    p.set_cue_routing(CueRouting::Split);
    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let out = DjOutputs {
        main: Box::new(producer),
        main_channels: 2,
        phones: None,
    };
    let runtime = DjRuntime::start(engine, out, 9_600);
    let start = Instant::now();
    while consumer.slots() < 19_200 && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime.stop();
    let chunk = consumer.read_chunk(19_200).expect("audio");
    let (a, b) = chunk.as_slices();
    let audio: Vec<f32> = a.iter().chain(b).copied().collect();
    let left: Vec<f32> = audio.iter().step_by(2).copied().collect();
    let right: Vec<f32> = audio.iter().skip(1).step_by(2).copied().collect();
    let hz = |s: &[f32]| {
        s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32 * 48_000.0
            / s.len() as f32
    };
    assert!(
        (hz(&left[2_000..]) - 300.0).abs() < 10.0,
        "left = master (deck A) {}",
        hz(&left[2_000..])
    );
    assert!(
        (hz(&right[2_000..]) - 900.0).abs() < 10.0,
        "right = cue (deck B) {}",
        hz(&right[2_000..])
    );
}
