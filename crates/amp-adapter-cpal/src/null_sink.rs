//! Hardware-free null and capture audio sinks for deterministic CI and headless testing.

#![forbid(unsafe_code)]

use amp_ports::OutputClock;
use rtrb::Consumer;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Samples recorded by a capture sink (see [`super::CpalAudioSink::new_capture`]).
#[derive(Clone, Default)]
pub struct CapturedOutput(pub(crate) Arc<Mutex<Vec<f32>>>);

impl CapturedOutput {
    /// Copy of every sample drained so far, interleaved.
    pub fn samples(&self) -> Vec<f32> {
        self.0.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Number of samples drained so far.
    pub fn len(&self) -> usize {
        self.0.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears recorded samples.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.0.lock() {
            guard.clear();
        }
    }

    /// Largest absolute sample value drained so far (0.0 for silence).
    pub fn peak(&self) -> f32 {
        self.0
            .lock()
            .map(|s| s.iter().fold(0.0f32, |m, x| m.max(x.abs())))
            .unwrap_or(0.0)
    }
}

/// Drains `consumer` at real-time pace on a background thread until the producer is dropped.
pub(crate) fn spawn_realtime_drain(
    mut consumer: Consumer<f32>,
    sample_rate: u32,
    channels: u16,
    clock: Arc<OutputClock>,
    capture: Option<CapturedOutput>,
) {
    let ch = usize::from(channels.max(1));
    let per_sec = sample_rate as f64 * ch as f64;
    thread::Builder::new()
        .name("amp-null-sink".to_string())
        .spawn(move || {
            let mut last_tick = std::time::Instant::now();
            let mut frac_samples = 0.0f64;
            while !consumer.is_abandoned() {
                let now = std::time::Instant::now();
                let dt = now.duration_since(last_tick).as_secs_f64();
                last_tick = now;

                // 1. Discard any stale-epoch samples without recording them
                let popped = clock.total_popped();
                let flush_until = clock.flush_until();
                if popped < flush_until {
                    let stale = (flush_until - popped) as usize;
                    let to_drop = (stale.min(consumer.slots())) / ch * ch;
                    if to_drop > 0 {
                        if let Ok(chunk) = consumer.read_chunk(to_drop) {
                            chunk.commit_all();
                        }
                        clock.advance_popped(to_drop);
                    }
                }

                // 2. Drain due frames at real-time rate, respecting pause & 5 ms fade
                frac_samples += dt * per_sec;
                let due_frames = (frac_samples as usize) / ch;
                let due_samples = due_frames * ch;
                frac_samples -= due_samples as f64;

                let avail_frames = consumer.slots() / ch;
                let frames_to_process = avail_frames.min(due_frames);
                if frames_to_process > 0 {
                    let mut newly_popped = 0usize;
                    let mut recorded = Vec::new();
                    for _ in 0..frames_to_process {
                        let Some(gain) = clock.next_frame_gain() else {
                            frac_samples = 0.0;
                            break;
                        };
                        let mut mono_val = 0.0f32;
                        for c in 0..ch {
                            let s = consumer.pop().unwrap_or(0.0) * gain;
                            newly_popped += 1;
                            mono_val = s;
                            if c < 2 {
                                clock.write_tap_sample(s);
                            }
                            if capture.is_some() {
                                recorded.push(s);
                            }
                        }
                        if ch == 1 {
                            clock.write_tap_sample(mono_val);
                        }
                    }
                    if newly_popped > 0 {
                        clock.advance_popped(newly_popped);
                    }
                    if let (Some(cap), false) = (&capture, recorded.is_empty()) {
                        if let Ok(mut buf) = cap.0.lock() {
                            buf.extend_from_slice(&recorded);
                        }
                    }
                }
                // About a sound card's period (512 frames at 48 kHz). Waking every 2 ms cost 0.5%
                // CPU while idle and made headless measurements worse than real devices.
                thread::sleep(Duration::from_millis(10));
            }
        })
        .ok();
}
