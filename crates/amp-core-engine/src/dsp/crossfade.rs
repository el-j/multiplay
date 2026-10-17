//! Interpolated dynamic crossfade DSP.
//! Provides linear and constant acoustic energy (equal power) transitions.

#![forbid(unsafe_code)]

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfadeCurve {
    Linear,
    EqualPower,
}

pub struct Crossfader {
    curve: CrossfadeCurve,
    duration_frames: usize,
    current_frame: usize,
    active: bool,
}

impl Crossfader {
    pub fn new(curve: CrossfadeCurve, duration_frames: usize) -> Self {
        Self {
            curve,
            duration_frames: duration_frames.max(1),
            current_frame: 0,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.current_frame = 0;
        self.active = true;
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }

    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Computes normalized gain coefficients: `(gain_outgoing, gain_incoming)`.
    #[inline(always)]
    pub fn step_gains(&mut self) -> (f32, f32) {
        if !self.active {
            return (1.0, 0.0);
        }

        let progress = (self.current_frame as f32 / self.duration_frames as f32).clamp(0.0, 1.0);
        self.current_frame += 1;

        if self.current_frame >= self.duration_frames {
            self.active = false;
        }

        match self.curve {
            CrossfadeCurve::Linear => {
                let out_gain = 1.0 - progress;
                let in_gain = progress;
                (out_gain, in_gain)
            }
            CrossfadeCurve::EqualPower => {
                let angle = progress * (PI / 2.0);
                (angle.cos(), angle.sin())
            }
        }
    }

    /// Blends interleaved stereo frames in-place with zero dynamic heap allocation.
    pub fn process_interleaved_blend(
        &mut self,
        outgoing: &[f32],
        incoming: &[f32],
        out_dest: &mut [f32],
    ) {
        let frames = (outgoing.len().min(incoming.len()).min(out_dest.len())) / 2;

        for f in 0..frames {
            let (g_out, g_in) = self.step_gains();
            let idx_l = f * 2;
            let idx_r = f * 2 + 1;

            out_dest[idx_l] = outgoing[idx_l] * g_out + incoming[idx_l] * g_in;
            out_dest[idx_r] = outgoing[idx_r] * g_out + incoming[idx_r] * g_in;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfade_linear_curve() {
        let mut xfader = Crossfader::new(CrossfadeCurve::Linear, 100);
        assert!(!xfader.is_active());
        assert_eq!(xfader.step_gains(), (1.0, 0.0));

        xfader.trigger();
        assert!(xfader.is_active());

        // First frame
        let (out_start, in_start) = xfader.step_gains();
        assert!((out_start - 1.0).abs() < 1e-4);
        assert!((in_start - 0.0).abs() < 1e-4);

        // Step to midpoint
        for _ in 1..50 {
            let _ = xfader.step_gains();
        }
        let (out_mid, in_mid) = xfader.step_gains();
        assert!((out_mid - 0.5).abs() < 0.05);
        assert!((in_mid - 0.5).abs() < 0.05);

        // Step to end
        while xfader.is_active() {
            let _ = xfader.step_gains();
        }
        assert!(!xfader.is_active());
        assert_eq!(xfader.step_gains(), (1.0, 0.0));
    }

    #[test]
    fn test_crossfade_equal_power_energy_sum() {
        let mut xfader = Crossfader::new(CrossfadeCurve::EqualPower, 500);
        xfader.trigger();

        while xfader.is_active() {
            let (g_out, g_in) = xfader.step_gains();
            // Equal power invariant: g1^2 + g2^2 should always equal 1.0
            let power_sum = g_out * g_out + g_in * g_in;
            assert!(
                (power_sum - 1.0).abs() < 1e-4,
                "Power sum mismatch: {power_sum}"
            );
        }
    }

    #[test]
    fn test_crossfade_interleaved_blend() {
        let mut xfader = Crossfader::new(CrossfadeCurve::Linear, 4);
        xfader.trigger();

        let outgoing = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]; // 4 stereo frames
        let incoming = [2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
        let mut dest = [0.0; 8];

        xfader.process_interleaved_blend(&outgoing, &incoming, &mut dest);
        assert!(dest[0] >= 1.0);
        assert!(dest[6] > dest[0]); // Later frames have more incoming weight
    }
}
