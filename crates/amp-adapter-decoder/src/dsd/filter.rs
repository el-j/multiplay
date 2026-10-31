//! DSD to PCM conversion in three decimating stages (DSD64: 2.8224 MHz → 88.2 kHz):
//!
//! 1. a 96-tap FIR on the 1-bit stream, one output per byte (÷8 → 352.8 kHz), evaluated through
//!    small byte lookup tables (12 KB, cache resident);
//! 2. a half-band FIR ÷2 → 176.4 kHz;
//! 3. a half-band FIR ÷2 → 88.2 kHz, flat to 24 kHz.
//!
//! Each stage only has to keep DSD's ultrasonic noise out of the band that stays audible after
//! the next one; what remains above 24 kHz is removed by the engine's output resampler.

#![forbid(unsafe_code)]

use std::f64::consts::PI;
use std::sync::OnceLock;

/// DSD bytes consumed per PCM output sample (32 bits).
pub const BYTES_PER_OUTPUT: usize = 4;
/// Idle DSD pattern (equal ones and zeros), used as silence before the stream starts.
pub const DSD_SILENCE: u8 = 0x69;
/// Bytes in the first-stage window (96 taps).
pub const TAP_BYTES: usize = 12;
/// Output samples a fresh filter needs before it is fully settled.
pub const WARMUP_OUTPUTS: usize = 32;

const STAGE2_TAPS: usize = 15;
const STAGE3_TAPS: usize = 39;

/// Blackman-windowed sinc low-pass with unity DC gain; `cutoff` relative to the input rate.
fn lowpass(taps: usize, cutoff: f64) -> Vec<f64> {
    let mid = (taps - 1) as f64 / 2.0;
    let mut h: Vec<f64> = (0..taps)
        .map(|n| {
            let x = n as f64 - mid;
            let sinc = if x == 0.0 {
                2.0 * cutoff
            } else {
                (2.0 * PI * cutoff * x).sin() / (PI * x)
            };
            let w = 0.42 - 0.5 * (2.0 * PI * n as f64 / (taps - 1) as f64).cos()
                + 0.08 * (4.0 * PI * n as f64 / (taps - 1) as f64).cos();
            sinc * w
        })
        .collect();
    let sum: f64 = h.iter().sum();
    h.iter_mut().for_each(|c| *c /= sum);
    h
}

struct Coefficients {
    /// `stage1[j][byte]`: contribution of `byte` (MSB = oldest bit) at byte position `j`.
    stage1: Vec<[f32; 256]>,
    /// Non-zero half-band taps as (window index, coefficient).
    stage2: Vec<(usize, f32)>,
    stage3: Vec<f32>,
}

fn coefficients() -> &'static Coefficients {
    static COEFFS: OnceLock<Coefficients> = OnceLock::new();
    COEFFS.get_or_init(|| {
        // Stage 1 at 2.8224 MHz: keep < 60 kHz; aliases from ÷8 land above 120 kHz.
        let h = lowpass(TAP_BYTES * 8, 60_000.0 / 2_822_400.0);
        let stage1 = (0..TAP_BYTES)
            .map(|j| {
                let mut t = [0.0f32; 256];
                for (byte, slot) in t.iter_mut().enumerate() {
                    *slot = (0..8)
                        .map(|bit| {
                            let one = (byte >> (7 - bit)) & 1 == 1;
                            h[j * 8 + bit] * if one { 1.0 } else { -1.0 }
                        })
                        .sum::<f64>() as f32;
                }
                t
            })
            .collect();
        let to_f32 = |v: Vec<f64>| v.into_iter().map(|c| c as f32).collect();
        Coefficients {
            stage1,
            // 352.8 kHz: half-band around 88.2 kHz.
            stage2: lowpass(STAGE2_TAPS, 0.25)
                .into_iter()
                .enumerate()
                .filter(|(_, h)| h.abs() > 1e-9)
                .map(|(i, h)| (i, h as f32))
                .collect(),
            // 176.4 kHz: flat to 24 kHz, aliases of ÷2 stay above 24 kHz.
            stage3: to_f32(lowpass(STAGE3_TAPS, 34_000.0 / 176_400.0)),
        }
    })
}

/// A FIR history kept twice in a row, so the current window is always one contiguous slice
/// (no shifting, no wrap-around per sample).
#[derive(Clone)]
struct Fir<T: Copy, const N2: usize> {
    x: [T; N2],
    pos: usize,
}

impl<T: Copy, const N2: usize> Fir<T, N2> {
    const N: usize = N2 / 2;

    fn new(fill: T) -> Self {
        Self {
            x: [fill; N2],
            pos: 0,
        }
    }

    fn push(&mut self, v: T) {
        self.x[self.pos] = v;
        self.x[self.pos + Self::N] = v;
        self.pos = (self.pos + 1) % Self::N;
    }

    /// Oldest to newest.
    fn window(&self) -> &[T] {
        &self.x[self.pos..self.pos + Self::N]
    }
}

/// One channel's filter state across all three stages.
#[derive(Clone)]
pub struct Channel {
    s1: Fir<u8, { TAP_BYTES * 2 }>,
    s2: Fir<f32, { STAGE2_TAPS * 2 }>,
    s3: Fir<f32, { STAGE3_TAPS * 2 }>,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            s1: Fir::new(DSD_SILENCE),
            s2: Fir::new(0.0),
            s3: Fir::new(0.0),
        }
    }
}

impl Channel {
    /// Pushes one output's worth of bytes (MSB first) and returns the 88.2 kHz sample.
    pub fn push(&mut self, bytes: [u8; BYTES_PER_OUTPUT]) -> f32 {
        let c = coefficients();
        for pair in bytes.as_chunks::<2>().0 {
            for &b in pair {
                self.s1.push(b);
                let v: f32 = self
                    .s1
                    .window()
                    .iter()
                    .zip(&c.stage1)
                    .map(|(&b, t)| t[usize::from(b)])
                    .sum();
                self.s2.push(v);
            }
            // Half-band: only the non-zero taps.
            let w = self.s2.window();
            let v: f32 = c.stage2.iter().map(|&(i, h)| w[i] * h).sum();
            self.s3.push(v);
        }
        self.s3
            .window()
            .iter()
            .zip(&c.stage3)
            .map(|(x, h)| x * h)
            .sum()
    }
}
