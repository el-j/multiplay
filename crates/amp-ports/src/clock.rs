//! Lock-free output clock, epoch flush coordinator, and speaker-aligned visualizer tap.
//!
//! Shared between the core playback engine producer and the real-time audio callback (or test
//! capture drain) with zero allocations, zero locks, and zero system calls on the RT path.

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Number of interleaved samples retained in the speaker-aligned visualizer tap (power of two).
pub const OUTPUT_TAP_SAMPLES: usize = 2048;
const TAP_MASK: usize = OUTPUT_TAP_SAMPLES - 1;

/// Lock-free telemetry and epoch synchronization shared between producer and RT audio consumer.
pub struct OutputClock {
    epoch: AtomicU64,
    total_pushed_samples: AtomicU64,
    total_popped_samples: AtomicU64,
    flush_until_sample: AtomicU64,
    epoch_start_sample: AtomicU64,
    seek_base_micros: AtomicU64,
    sample_rate: AtomicU32,
    source_channels: AtomicU16,
    paused: AtomicBool,
    pause_fade_total: AtomicU32,
    pause_fade_remaining: AtomicU32,
    tap_samples: [AtomicU32; OUTPUT_TAP_SAMPLES],
    tap_write_pos: AtomicUsize,
}

impl std::fmt::Debug for OutputClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputClock")
            .field("epoch", &self.epoch())
            .field("position", &self.current_position())
            .field("paused", &self.is_paused())
            .finish()
    }
}

impl OutputClock {
    /// Creates a new output clock for a sink running at `sample_rate` with `source_channels`
    /// interleaved channels in its ring buffer (normally 2 for stereo).
    pub fn new(sample_rate: u32, source_channels: u16) -> Self {
        Self {
            epoch: AtomicU64::new(0),
            total_pushed_samples: AtomicU64::new(0),
            total_popped_samples: AtomicU64::new(0),
            flush_until_sample: AtomicU64::new(0),
            epoch_start_sample: AtomicU64::new(0),
            seek_base_micros: AtomicU64::new(0),
            sample_rate: AtomicU32::new(sample_rate.max(1)),
            source_channels: AtomicU16::new(source_channels.max(1)),
            paused: AtomicBool::new(false),
            pause_fade_total: AtomicU32::new(0),
            pause_fade_remaining: AtomicU32::new(0),
            tap_samples: std::array::from_fn(|_| AtomicU32::new(0)),
            tap_write_pos: AtomicUsize::new(0),
        }
    }

    /// Updates sample rate and source channel count.
    pub fn set_format(&self, sample_rate: u32, source_channels: u16) {
        self.sample_rate
            .store(sample_rate.max(1), Ordering::Release);
        self.source_channels
            .store(source_channels.max(1), Ordering::Release);
    }

    /// Current generation epoch counter.
    #[inline]
    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    /// Records `count` newly pushed interleaved samples from the producer.
    #[inline]
    pub fn record_pushed(&self, count: usize) -> u64 {
        self.total_pushed_samples
            .fetch_add(count as u64, Ordering::Release)
            + count as u64
    }

    /// Total interleaved samples ever pushed into the ring buffer.
    #[inline]
    pub fn total_pushed(&self) -> u64 {
        self.total_pushed_samples.load(Ordering::Acquire)
    }

    /// Total interleaved samples popped or flushed by the output consumer.
    #[inline]
    pub fn total_popped(&self) -> u64 {
        self.total_popped_samples.load(Ordering::Acquire)
    }

    /// Sample boundary prior to which all queued samples belong to a stale epoch and must be dropped.
    #[inline]
    pub fn flush_until(&self) -> u64 {
        self.flush_until_sample.load(Ordering::Acquire)
    }

    /// Number of active (unflushed, unpopped) interleaved samples currently queued in the ring.
    #[inline]
    pub fn queued_samples(&self) -> u64 {
        let pushed = self.total_pushed_samples.load(Ordering::Acquire);
        let popped = self
            .total_popped_samples
            .load(Ordering::Acquire)
            .max(self.flush_until_sample.load(Ordering::Acquire));
        pushed.saturating_sub(popped)
    }

    /// Advances the consumer's monotonic sample counter.
    #[inline]
    pub fn advance_popped(&self, count: usize) -> u64 {
        self.total_popped_samples
            .fetch_add(count as u64, Ordering::Release)
            + count as u64
    }

    /// Flushes all currently queued samples in the ring buffer and resets the output clock
    /// to `seek_base` (used on stop, seek, and explicit track changes).
    pub fn flush_and_reset(&self, seek_base: Duration) -> u64 {
        let pushed = self.total_pushed_samples.load(Ordering::Acquire);
        self.seek_base_micros
            .store(seek_base.as_micros() as u64, Ordering::Release);
        self.epoch_start_sample.store(pushed, Ordering::Release);
        self.flush_until_sample.store(pushed, Ordering::Release);
        self.clear_tap();
        self.epoch.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Marks the boundary of a seamless gapless or crossfaded track transition without
    /// discarding already-queued samples from the outgoing track.
    pub fn mark_seamless_transition(&self, new_base: Duration) {
        let pushed = self.total_pushed_samples.load(Ordering::Acquire);
        self.seek_base_micros
            .store(new_base.as_micros() as u64, Ordering::Release);
        self.epoch_start_sample.store(pushed, Ordering::Release);
    }

    /// Requests a click-free pause fade-out over `fade_frames` frames (typically 5 ms).
    pub fn request_pause(&self, fade_frames: u32) {
        self.pause_fade_total.store(fade_frames, Ordering::Release);
        self.pause_fade_remaining
            .store(fade_frames, Ordering::Release);
        self.paused.store(true, Ordering::Release);
    }

    /// Immediately pauses output without a fade (used on stop).
    pub fn pause_immediately(&self) {
        self.pause_fade_total.store(0, Ordering::Release);
        self.pause_fade_remaining.store(0, Ordering::Release);
        self.paused.store(true, Ordering::Release);
        self.clear_tap();
    }

    /// Resumes output consumption.
    pub fn resume(&self) {
        self.pause_fade_remaining.store(0, Ordering::Release);
        self.paused.store(false, Ordering::Release);
    }

    /// True when the sink has been asked to pause.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// Computes the gain multiplier for the next output frame during a pause fade-out.
    /// Returns `Some(gain)` if the frame should be popped and scaled, or `None` if the fade is
    /// complete and the consumer should output silence without popping from the ring buffer.
    #[inline]
    pub fn next_frame_gain(&self) -> Option<f32> {
        if !self.paused.load(Ordering::Acquire) {
            return Some(1.0);
        }
        let rem = self.pause_fade_remaining.load(Ordering::Relaxed);
        if rem == 0 {
            return None;
        }
        let total = self.pause_fade_total.load(Ordering::Relaxed).max(1);
        self.pause_fade_remaining
            .store(rem.saturating_sub(1), Ordering::Relaxed);
        // Smooth cosine fade-out from 1.0 down to 0.0
        let progress = 1.0 - (rem as f32 / total as f32);
        let gain = 0.5 * (1.0 + (progress * std::f32::consts::PI).cos());
        Some(gain)
    }

    /// Speaker-aligned playback position computed from the seek base plus frames actually
    /// consumed by the audio output callback.
    pub fn current_position(&self) -> Duration {
        let popped = self.total_popped_samples.load(Ordering::Acquire);
        let start = self.epoch_start_sample.load(Ordering::Acquire);
        let consumed_samples = popped.saturating_sub(start);
        let ch = u64::from(self.source_channels.load(Ordering::Relaxed).max(1));
        let sr = f64::from(self.sample_rate.load(Ordering::Relaxed).max(1));
        let consumed_frames = consumed_samples / ch;
        let base = Duration::from_micros(self.seek_base_micros.load(Ordering::Acquire));
        base + Duration::from_secs_f64(consumed_frames as f64 / sr)
    }

    /// Writes a slice of interleaved stereo samples that are leaving the speaker into the
    /// lock-free visualizer tap. Real-time safe (no locks, no allocations).
    #[inline]
    pub fn write_tap(&self, samples: &[f32]) {
        let mut pos = self.tap_write_pos.load(Ordering::Relaxed);
        for &s in samples {
            self.tap_samples[pos & TAP_MASK].store(s.to_bits(), Ordering::Relaxed);
            pos = pos.wrapping_add(1);
        }
        self.tap_write_pos.store(pos, Ordering::Release);
    }

    /// Writes a single sample into the lock-free visualizer tap. Real-time safe.
    #[inline]
    pub fn write_tap_sample(&self, sample: f32) {
        let pos = self.tap_write_pos.load(Ordering::Relaxed);
        self.tap_samples[pos & TAP_MASK].store(sample.to_bits(), Ordering::Relaxed);
        self.tap_write_pos
            .store(pos.wrapping_add(1), Ordering::Release);
    }

    /// Reads the most recent `out.len()` samples heard at the speaker from the lock-free tap.
    /// Fills `out` with silence when paused (after fade) or when no samples have played yet.
    pub fn read_tap(&self, out: &mut [f32]) {
        if self.is_paused() && self.pause_fade_remaining.load(Ordering::Relaxed) == 0 {
            out.fill(0.0);
            return;
        }
        let end = self.tap_write_pos.load(Ordering::Acquire);
        let len = out.len().min(OUTPUT_TAP_SAMPLES);
        let start = end.wrapping_sub(len);
        for (i, slot) in out.iter_mut().take(len).enumerate() {
            let bits = self.tap_samples[start.wrapping_add(i) & TAP_MASK].load(Ordering::Relaxed);
            *slot = f32::from_bits(bits);
        }
        for slot in out.iter_mut().skip(len) {
            *slot = 0.0;
        }
    }

    /// Zeros the visualizer tap buffer.
    pub fn clear_tap(&self) {
        for slot in &self.tap_samples {
            slot.store(0.0f32.to_bits(), Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_clock_position_tracks_consumed_frames_and_seek_base() {
        let clock = OutputClock::new(48_000, 2);
        assert_eq!(clock.current_position(), Duration::ZERO);

        // Producer pushes 48,000 stereo frames (96,000 samples = 1.0 s), but consumer has only
        // popped 24,000 stereo frames (48,000 samples = 0.5 s).
        clock.record_pushed(96_000);
        clock.advance_popped(48_000);
        let pos = clock.current_position();
        assert!((pos.as_secs_f64() - 0.5).abs() < 1e-6);

        // Seek to 10.0 s: flushes remaining 48,000 stale samples and sets seek base to 10.0 s.
        clock.flush_and_reset(Duration::from_secs(10));
        assert_eq!(clock.flush_until(), 96_000);
        assert_eq!(clock.current_position(), Duration::from_secs(10));

        // Consumer discards the 48,000 stale samples (reaching 96,000), then pops 9,600 new samples (0.1 s).
        clock.advance_popped(48_000);
        assert_eq!(clock.current_position(), Duration::from_secs(10));
        clock.record_pushed(19_200);
        clock.advance_popped(9_600);
        assert!((clock.current_position().as_secs_f64() - 10.1).abs() < 1e-6);
    }

    #[test]
    fn pause_fade_ramps_smoothly_to_silence_and_resume_restores_unity() {
        let clock = OutputClock::new(48_000, 2);
        assert_eq!(clock.next_frame_gain(), Some(1.0));

        clock.request_pause(4);
        let g0 = clock.next_frame_gain().unwrap();
        let g1 = clock.next_frame_gain().unwrap();
        let g2 = clock.next_frame_gain().unwrap();
        let g3 = clock.next_frame_gain().unwrap();
        assert!(g0 >= g1 && g1 >= g2 && g2 >= g3);
        assert!(g3 < 0.2);
        assert_eq!(clock.next_frame_gain(), None);

        clock.resume();
        assert_eq!(clock.next_frame_gain(), Some(1.0));
    }

    #[test]
    fn speaker_tap_returns_latest_samples_and_zeros_when_paused() {
        let clock = OutputClock::new(48_000, 2);
        clock.write_tap(&[0.25, -0.25, 0.5, -0.5]);
        let mut out = [0.0f32; 4];
        clock.read_tap(&mut out);
        assert_eq!(out, [0.25, -0.25, 0.5, -0.5]);

        clock.pause_immediately();
        clock.read_tap(&mut out);
        assert_eq!(out, [0.0; 4]);
    }
}
