//! Ring-buffer draining and speaker-aligned clock/visualizer synchronization helpers.

#![forbid(unsafe_code)]

use super::PlayerEngine;
use crate::dsp::SpectrumVisualizer;
use crate::events::PlayerEvent;
use amp_domain::PlaybackState;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Below this much queued audio (about 43 ms at 48 kHz) a network stream counts as buffering.
const LOW_WATER_FRAMES: usize = 2048;

/// Network streams that cannot keep up (TASK-186): when the output buffer runs nearly dry, hold
/// the output (a short fade, then silence) and say "Buffering" instead of stuttering on as
/// "Playing"; carry on once the output ring is three quarters full. Local files are left alone.
pub(crate) fn watch_stream_buffer(driver: &PlayerEngine) {
    if !driver.is_playing.load(Ordering::Acquire) {
        return;
    }
    let network = driver
        .current_track
        .lock()
        .ok()
        .and_then(|t| {
            t.as_ref()
                .map(|t| !matches!(t.source, amp_domain::AudioSource::LocalFile { .. }))
        })
        .unwrap_or(false);
    if !network {
        return;
    }
    let Some((queued, capacity)) = driver.producer.try_lock().ok().map(|p| {
        let cap = p.buffer().capacity() / 2;
        (cap - p.slots() / 2, cap)
    }) else {
        return;
    };
    // Resume at three quarters of the output ring (150 ms by default); the stream's own
    // download buffer upstream holds far more.
    let high = (capacity * 3 / 4).max(LOW_WATER_FRAMES + 1);
    let state = driver.playback_state();
    // The user did not pause (`is_playing`), so a paused clock means the output is held for
    // buffering. Resume on the clock, not the state: the state may have been set elsewhere.
    if driver.clock.is_paused() {
        if queued >= high {
            driver.clock.resume();
            driver.set_state_and_emit(PlaybackState::Playing);
        } else if matches!(state, PlaybackState::Buffering { .. }) {
            let progress_percent = u8::try_from(queued * 100 / high).unwrap_or(99);
            driver.set_state_and_emit(PlaybackState::Buffering { progress_percent });
        }
    } else if state == PlaybackState::Playing && queued < LOW_WATER_FRAMES {
        let fade = ((driver.sample_rate() * 5) / 1000).max(32);
        driver.clock.request_pause(fade);
        driver.set_state_and_emit(PlaybackState::Buffering {
            progress_percent: 0,
        });
    }
}

pub(crate) fn drain_pending_to_ring(driver: &PlayerEngine, pending_out: &mut VecDeque<f32>) {
    while pending_out.len() >= 2 {
        if !driver.is_playing.load(Ordering::Acquire)
            || driver.track_change_signal.load(Ordering::Acquire)
            || driver.seek_requested.lock().unwrap().is_some()
        {
            break;
        }
        let pushed = if let Ok(mut prod) = driver.producer.try_lock() {
            let count = (prod.slots() / 2 * 2).min(pending_out.len() / 2 * 2);
            for _ in 0..count {
                if let Some(s) = pending_out.pop_front() {
                    let _ = prod.push(s);
                }
            }
            count
        } else {
            0
        };
        if pushed > 0 {
            driver.clock.record_pushed(pushed);
        } else {
            // A full ring while buffering means enough is buffered: resume, or this loop would
            // wait for space that the held output never makes.
            watch_stream_buffer(driver);
            driver.wait_for_wake(Duration::from_millis(2));
        }
    }
}

pub(crate) fn update_clock_position_and_tap(
    driver: &PlayerEngine,
    tap_visualizer: &mut SpectrumVisualizer,
    tap_buf: &mut [f32],
) {
    let audible = driver.clock.current_position();
    let prev = {
        let mut guard = driver.elapsed.lock().unwrap();
        let prev = *guard;
        *guard = audible;
        prev
    };
    if audible.abs_diff(prev) >= Duration::from_millis(50) {
        let total = *driver.total_duration.lock().unwrap();
        driver.emit(PlayerEvent::PositionUpdated {
            elapsed: audible,
            total,
        });
    }
    let popped_since_flush = driver
        .clock
        .total_popped()
        .saturating_sub(driver.clock.flush_until());
    if popped_since_flush >= 512 {
        driver.clock.read_tap(tap_buf);
        tap_visualizer.push_samples(tap_buf);
        let bands = tap_visualizer.spectrum_bands_throttled();
        if let Ok(mut spec) = driver.visualizer_spectrum.try_lock() {
            *spec = bands;
        }
        if let Ok(mut wave) = driver.visualizer_waveform.try_lock() {
            let step = (tap_buf.len() / 64).max(1);
            for (dst_i, s) in wave.iter_mut().enumerate() {
                *s = tap_buf[(dst_i * step).min(tap_buf.len().saturating_sub(1))];
            }
        }
        if audible.abs_diff(prev) >= Duration::from_millis(50) {
            driver.emit(PlayerEvent::SpectrumDataReady(bands.to_vec()));
        }
    }
}

pub(crate) fn update_visualizer_fallback(
    driver: &PlayerEngine,
    processed: &[f32],
    bands: [f32; 16],
) {
    let popped_since_flush = driver
        .clock
        .total_popped()
        .saturating_sub(driver.clock.flush_until());
    if popped_since_flush < 512 {
        if let Ok(mut spec) = driver.visualizer_spectrum.try_lock() {
            *spec = bands;
        }
        if let Ok(mut wave) = driver.visualizer_waveform.try_lock() {
            let step = (processed.len() / 64).max(1);
            for (dst_i, s) in wave.iter_mut().enumerate() {
                *s = processed[(dst_i * step).min(processed.len().saturating_sub(1))];
            }
        }
    }
}

/// Used by `DjHost` to push rendered DJ frames and advance `OutputClock`.
pub(crate) fn push_samples_with_clock(driver: &PlayerEngine, processed: &[f32], bands: [f32; 16]) {
    if let Ok(mut spec) = driver.visualizer_spectrum.try_lock() {
        *spec = bands;
    }
    if let Ok(mut wave) = driver.visualizer_waveform.try_lock() {
        let step = (processed.len() / 64).max(1);
        for (dst_i, s) in wave.iter_mut().enumerate() {
            *s = processed[(dst_i * step).min(processed.len().saturating_sub(1))];
        }
    }
    if let Ok(mut prod) = driver.producer.try_lock() {
        let mut pushed = 0;
        while pushed < processed.len() {
            match prod.push(processed[pushed]) {
                Ok(()) => pushed += 1,
                Err(_) => std::thread::sleep(Duration::from_millis(2)),
            }
        }
        driver.clock.record_pushed(pushed);
    }
}
