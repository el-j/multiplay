//! Real-time safe CPAL audio callback filler, N-channel mapping, and TPDF dither (`TASK-132`).

#![forbid(unsafe_code)]

use amp_ports::OutputClock;
use rtrb::Consumer;

/// Real-time safe, zero-allocation xorshift32 TPDF dither generator (`TASK-132`).
#[derive(Debug, Clone, Copy)]
pub struct TpdfDither {
    state: u32,
}

impl Default for TpdfDither {
    fn default() -> Self {
        Self::new(0x9E37_79B9)
    }
}

impl TpdfDither {
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    #[inline]
    fn next_uniform(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        (x as f32) * (1.0 / 4_294_967_296.0) - 0.5
    }

    /// Generates a triangular probability density function (TPDF) dither value in `[-1.0, 1.0]` LSB.
    #[inline]
    pub fn next_tpdf(&mut self) -> f32 {
        self.next_uniform() + self.next_uniform()
    }
}

#[inline]
pub fn f32_to_i16_dithered(sample: f32, dither: &mut TpdfDither) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    let scaled = clamped * 32_767.0 + dither.next_tpdf();
    scaled.round().clamp(-32_768.0, 32_767.0) as i16
}

#[inline]
pub fn f32_to_u16_dithered(sample: f32, dither: &mut TpdfDither) -> u16 {
    let clamped = sample.clamp(-1.0, 1.0);
    let scaled = (clamped * 32_767.0) + 32_768.0 + dither.next_tpdf();
    scaled.round().clamp(0.0, 65_535.0) as u16
}

#[inline]
pub fn f32_to_i32_dithered(sample: f32, dither: &mut TpdfDither) -> i32 {
    let clamped = f64::from(sample.clamp(-1.0, 1.0));
    let scaled = clamped * 2_147_483_647.0 + f64::from(dither.next_tpdf()) * 256.0;
    scaled.round().clamp(-2_147_483_648.0, 2_147_483_647.0) as i32
}

/// Fills a device buffer of `device_ch` interleaved channels from a queue holding
/// `source_ch`-channel frames.
#[cfg(test)]
pub(crate) fn fill_output(
    output: &mut [f32],
    device_ch: usize,
    source_ch: usize,
    mut pop: impl FnMut() -> Option<f32>,
) {
    let (device_ch, source_ch) = (device_ch.max(1), source_ch.max(1));
    if device_ch == source_ch {
        for s in output.iter_mut() {
            *s = pop().unwrap_or(0.0);
        }
        return;
    }
    for frame in output.chunks_mut(device_ch) {
        frame.fill(0.0);
        let mut sum = 0.0;
        for c in 0..source_ch {
            let v = pop().unwrap_or(0.0);
            sum += v;
            if let Some(slot) = frame.get_mut(c) {
                *slot = v;
            }
        }
        if device_ch == 1 {
            frame[0] = sum / source_ch as f32;
        }
    }
}

/// Generic real-time callback filler supporting any sample format `T` (`f32`, `i16`, `u16`, `i32`)
/// with epoch flushing, 5 ms pause fade-out, N-channel mapping, and speaker tap (`TASK-132`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn fill_output_typed_with_clock<T: Copy>(
    output: &mut [T],
    device_ch: usize,
    source_ch: usize,
    consumer: &mut Consumer<f32>,
    clock: &OutputClock,
    dither: &mut TpdfDither,
    zero: T,
    convert: impl Fn(f32, &mut TpdfDither) -> T,
) {
    let (device_ch, source_ch) = (device_ch.max(1), source_ch.max(1));

    // 1. Discard stale-epoch samples in O(1)
    let popped = clock.total_popped();
    let flush_until = clock.flush_until();
    if popped < flush_until {
        let stale = (flush_until - popped) as usize;
        let to_drop = (stale.min(consumer.slots())) / source_ch * source_ch;
        if to_drop > 0 {
            if let Ok(chunk) = consumer.read_chunk(to_drop) {
                chunk.commit_all();
            }
            clock.advance_popped(to_drop);
        }
    }

    // 2. Render frames with pause fade-out, channel mapping, TPDF dither, and visualizer tap
    let mut newly_popped = 0usize;
    for frame in output.chunks_mut(device_ch) {
        let Some(gain) = clock.next_frame_gain() else {
            frame.fill(zero);
            continue;
        };
        if consumer.slots() < source_ch {
            frame.fill(zero);
            continue;
        };
        frame.fill(zero);
        let mut sum = 0.0f32;
        for c in 0..source_ch {
            let v = consumer.pop().unwrap_or(0.0) * gain;
            newly_popped += 1;
            sum += v;
            if c < 2 {
                clock.write_tap_sample(v);
            }
            if device_ch > 1 {
                if let Some(slot) = frame.get_mut(c) {
                    *slot = convert(v, dither);
                }
            }
        }
        if source_ch == 1 {
            clock.write_tap_sample(sum);
        }
        if device_ch == 1 {
            frame[0] = convert(sum / source_ch as f32, dither);
        }
    }
    if newly_popped > 0 {
        clock.advance_popped(newly_popped);
    }
}

/// Convenience wrapper for `f32` output buffers in tests.
#[cfg(test)]
pub(crate) fn fill_output_with_clock(
    output: &mut [f32],
    device_ch: usize,
    source_ch: usize,
    consumer: &mut Consumer<f32>,
    clock: &OutputClock,
) {
    let mut dither = TpdfDither::default();
    fill_output_typed_with_clock(
        output,
        device_ch,
        source_ch,
        consumer,
        clock,
        &mut dither,
        0.0f32,
        |s, _| s.clamp(-1.0, 1.0),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;
    use std::time::Duration;

    fn run(device: usize, source: usize, input: &[f32], frames: usize) -> Vec<f32> {
        let mut it = input.iter().copied();
        let mut out = vec![9.0; frames * device];
        fill_output(&mut out, device, source, || it.next());
        out
    }

    #[test]
    fn stereo_plays_on_the_first_two_channels_of_a_4_channel_device() {
        let out = run(4, 2, &[0.1, 0.2, 0.3, 0.4], 2);
        assert_eq!(out, vec![0.1, 0.2, 0.0, 0.0, 0.3, 0.4, 0.0, 0.0]);
    }

    #[test]
    fn stereo_is_averaged_on_a_mono_device() {
        let out = run(1, 2, &[0.2, 0.4, -0.2, 0.0], 2);
        assert!((out[0] - 0.3).abs() < 1e-6 && (out[1] + 0.1).abs() < 1e-6);
    }

    #[test]
    fn matching_layouts_pass_through_and_underruns_are_silent() {
        assert_eq!(run(2, 2, &[0.5, -0.5], 2), vec![0.5, -0.5, 0.0, 0.0]);
        assert_eq!(
            run(4, 4, &[1.0, 2.0, 3.0, 4.0], 1),
            vec![1.0, 2.0, 3.0, 4.0]
        );
    }

    #[test]
    fn callback_discards_stale_epoch_samples_before_playing_new_epoch() {
        let (mut prod, mut cons) = RingBuffer::<f32>::new(64);
        let clock = OutputClock::new(48_000, 2);

        for &s in &[0.9, 0.9, 0.8, 0.8] {
            prod.push(s).unwrap();
        }
        clock.record_pushed(4);

        clock.flush_and_reset(Duration::from_secs(5));
        for &s in &[-0.25, 0.25] {
            prod.push(s).unwrap();
        }
        clock.record_pushed(2);

        let mut out = vec![0.0f32; 4];
        fill_output_with_clock(&mut out, 2, 2, &mut cons, &clock);
        assert_eq!(out, vec![-0.25, 0.25, 0.0, 0.0]);
        assert_eq!(clock.total_popped(), 6);
    }

    #[test]
    fn format_tpdf_dither_and_typed_callback_convert_i16_u16_i32_and_8ch() {
        let mut dither = TpdfDither::new(12345);
        assert_eq!(f32_to_i16_dithered(1.0, &mut dither), i16::MAX);
        assert_eq!(f32_to_i16_dithered(-1.0, &mut dither), -32_767);
        assert!((f32_to_u16_dithered(0.0, &mut dither) as i32 - 32_768).abs() <= 2);
        assert!(f32_to_i32_dithered(0.5, &mut dither) > 1_000_000_000);

        let (mut prod, mut cons) = RingBuffer::<f32>::new(32);
        let clock = OutputClock::new(48_000, 2);
        prod.push(0.5).unwrap();
        prod.push(-0.5).unwrap();
        clock.record_pushed(2);

        let mut out_i16 = vec![99i16; 8];
        fill_output_typed_with_clock(
            &mut out_i16,
            8,
            2,
            &mut cons,
            &clock,
            &mut dither,
            0i16,
            f32_to_i16_dithered,
        );
        assert!(out_i16[0] > 16_000);
        assert!(out_i16[1] < -16_000);
        assert_eq!(&out_i16[2..8], &[0, 0, 0, 0, 0, 0]);
    }
}
