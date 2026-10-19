//! Integrated loudness after EBU R128 / ITU-R BS.1770: K-weighting, 400 ms blocks with 75 %
//! overlap, an absolute gate at -70 LUFS and a relative gate 10 LU below the ungated level.

#![forbid(unsafe_code)]

use crate::dsp::BiquadFilter;

const ABSOLUTE_GATE: f64 = -70.0;
const RELATIVE_GATE: f64 = -10.0;

/// Streaming loudness meter over a whole track.
#[derive(Debug)]
pub(crate) struct Meter {
    /// K-weighting per channel: high shelf, then high-pass.
    weighting: [[BiquadFilter; 2]; 2],
    step: usize,
    seen: usize,
    sum: f64,
    /// Mean square (left + right) of each 100 ms step.
    steps: Vec<f64>,
}

impl Meter {
    pub(crate) fn new(rate: u32) -> Self {
        let channel = k_weighting(f64::from(rate));
        Self {
            weighting: [channel; 2],
            step: (rate as usize / 10).max(1),
            seen: 0,
            sum: 0.0,
            steps: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, l: f32, r: f32) {
        let mut z = 0.0;
        for (ch, x) in self.weighting.iter_mut().zip([l, r]) {
            let shelved = ch[0].process_sample(x);
            let y = ch[1].process_sample(shelved);
            z += f64::from(y) * f64::from(y);
        }
        self.sum += z;
        self.seen += 1;
        if self.seen == self.step {
            self.steps.push(self.sum / self.step as f64);
            self.sum = 0.0;
            self.seen = 0;
        }
    }

    /// Integrated loudness in LUFS, `None` when every block is below the absolute gate.
    pub(crate) fn integrated(&self) -> Option<f64> {
        let blocks: Vec<f64> = self
            .steps
            .windows(4)
            .map(|w| w.iter().sum::<f64>() / 4.0)
            .collect();
        let lufs = |z: f64| -0.691 + 10.0 * z.log10();
        let mean = |zs: &mut dyn Iterator<Item = f64>| {
            let (sum, n) = zs.fold((0.0, 0usize), |(s, n), z| (s + z, n + 1));
            (n > 0).then(|| sum / n as f64)
        };
        let loud = |z: &&f64| **z > 0.0 && lufs(**z) > ABSOLUTE_GATE;
        let ungated = mean(&mut blocks.iter().filter(loud).copied())?;
        let gate = lufs(ungated) + RELATIVE_GATE;
        let gated = mean(
            &mut blocks
                .iter()
                .filter(loud)
                .copied()
                .filter(|z| lufs(*z) > gate),
        )?;
        Some(lufs(gated))
    }
}

/// The two BS.1770 K-weighting stages for `rate`, in the bilinear form libebur128 uses, which
/// reproduces the standard's 48 kHz coefficients exactly and adapts them to other rates.
fn k_weighting(rate: f64) -> [BiquadFilter; 2] {
    let (f0, gain_db, q) = (
        1_681.974_450_955_533,
        3.999_843_853_973_347,
        0.707_175_236_955_419_6,
    );
    let k = (std::f64::consts::PI * f0 / rate).tan();
    let vh = 10f64.powf(gain_db / 20.0);
    let vb = vh.powf(0.499_666_774_154_541_6);
    let a0 = 1.0 + k / q + k * k;
    let shelf = BiquadFilter::from_coefficients(
        ((vh + vb * k / q + k * k) / a0) as f32,
        (2.0 * (k * k - vh) / a0) as f32,
        ((vh - vb * k / q + k * k) / a0) as f32,
        (2.0 * (k * k - 1.0) / a0) as f32,
        ((1.0 - k / q + k * k) / a0) as f32,
    );
    let (f0, q) = (38.135_470_876_024_44, 0.500_327_037_323_877_3);
    let k = (std::f64::consts::PI * f0 / rate).tan();
    let a0 = 1.0 + k / q + k * k;
    let high_pass = BiquadFilter::from_coefficients(
        1.0,
        -2.0,
        1.0,
        (2.0 * (k * k - 1.0) / a0) as f32,
        ((1.0 - k / q + k * k) / a0) as f32,
    );
    [shelf, high_pass]
}
