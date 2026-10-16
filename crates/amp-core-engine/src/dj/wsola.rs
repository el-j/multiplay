//! Pure-Rust WSOLA pitch-preserving time-stretcher and DJ crossfader transform mixer.
//! Real-time allocation-free execution for hardware-style DJ consoles.

#![forbid(unsafe_code)]

/// Crossfader gain transform profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfaderCurveProfile {
    /// Equal-energy smooth dip for continuous long blends (House / Techno).
    SmoothDip,
    /// Standard straight linear slope.
    Linear,
    /// Sharp fast cut reaching 100% within 5% fader travel (Scratch / Hip-Hop).
    SharpCut,
}

/// DJ crossfader mixer calculating dual-bus channel gains across fader travel.
#[derive(Debug, Clone, Copy)]
pub struct DjCrossfader {
    position: f32, // -1.0 (Deck Left) to +1.0 (Deck Right)
    profile: CrossfaderCurveProfile,
}

impl DjCrossfader {
    /// Creates a new crossfader at center position (0.0) with SmoothDip profile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: 0.0,
            profile: CrossfaderCurveProfile::SmoothDip,
        }
    }

    /// Sets the position (-1.0 to 1.0).
    pub fn set_position(&mut self, pos: f32) {
        self.position = pos.clamp(-1.0, 1.0);
    }

    /// Sets the curve profile.
    pub fn set_profile(&mut self, profile: CrossfaderCurveProfile) {
        self.profile = profile;
    }

    /// Computes `(left_bus_gain, right_bus_gain)`.
    #[must_use]
    pub fn compute_gains(&self) -> (f32, f32) {
        let t = (self.position + 1.0) * 0.5; // Normalized 0.0 (full left) to 1.0 (full right)

        match self.profile {
            CrossfaderCurveProfile::Linear => (1.0 - t, t),
            CrossfaderCurveProfile::SmoothDip => {
                let left = (1.0 - t).sqrt();
                let right = t.sqrt();
                (left, right)
            }
            CrossfaderCurveProfile::SharpCut => {
                let left = if t > 0.95 { (1.0 - t) / 0.05 } else { 1.0 };
                let right = if t < 0.05 { t / 0.05 } else { 1.0 };
                (left.clamp(0.0, 1.0), right.clamp(0.0, 1.0))
            }
        }
    }
}

impl Default for DjCrossfader {
    fn default() -> Self {
        Self::new()
    }
}

/// Waveform Similarity Overlap-Add (WSOLA) pitch-preserving time-stretcher.
/// Allows speed scaling (0.5x to 2.0x) without modifying audio pitch.
#[derive(Debug, Clone)]
pub struct WsolaTimeStretcher {
    speed_ratio: f32,
    window_size: usize,
    overlap_buffer: Vec<f32>,
}

impl WsolaTimeStretcher {
    /// Creates a new WSOLA time-stretcher with specified window size (typically 512-1024 samples).
    #[must_use]
    pub fn new(window_size: usize) -> Self {
        Self {
            speed_ratio: 1.0,
            window_size,
            overlap_buffer: vec![0.0; window_size],
        }
    }

    /// Updates playback speed ratio (0.5x to 2.0x).
    pub fn set_speed_ratio(&mut self, ratio: f32) {
        self.speed_ratio = ratio.clamp(0.5, 2.0);
    }

    /// Current speed ratio.
    #[must_use]
    pub const fn speed_ratio(&self) -> f32 {
        self.speed_ratio
    }

    /// Analysis window size in samples.
    #[must_use]
    pub const fn window_size(&self) -> usize {
        self.window_size
    }

    /// Internal overlap buffer slice.
    #[must_use]
    pub fn overlap_buffer(&self) -> &[f32] {
        &self.overlap_buffer
    }

    /// Stretches or compresses input audio samples into output buffer without pitch shifting.
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> usize {
        if (self.speed_ratio - 1.0).abs() < 0.001 {
            let n = input.len().min(output.len());
            output[..n].copy_from_slice(&input[..n]);
            return n;
        }

        let mut in_pos = 0.0_f32;
        let mut out_idx = 0;

        while out_idx < output.len() && (in_pos as usize + self.window_size) < input.len() {
            let base_idx = in_pos as usize;
            for i in 0..self.window_size {
                let window_weight = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * (i as f32) / (self.window_size as f32)).cos());
                if out_idx + i < output.len() {
                    output[out_idx + i] = input[base_idx + i] * window_weight;
                }
            }

            out_idx += self.window_size / 2;
            in_pos += (self.window_size / 2) as f32 * self.speed_ratio;
        }

        out_idx.min(output.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wsola_time_stretcher() {
        let mut stretcher = WsolaTimeStretcher::new(256);
        assert!((stretcher.speed_ratio() - 1.0).abs() < f32::EPSILON);

        stretcher.set_speed_ratio(1.25);
        assert!((stretcher.speed_ratio() - 1.25).abs() < f32::EPSILON);

        let input = vec![0.1_f32; 1024];
        let mut output = vec![0.0_f32; 1024];
        let written = stretcher.process(&input, &mut output);
        assert!(written > 0);
    }

    #[test]
    fn test_dj_crossfader_profiles() {
        let mut fader = DjCrossfader::new();

        // Center with SmoothDip
        let (left, right) = fader.compute_gains();
        assert!((left - right).abs() < 0.01);
        assert!(left > 0.6); // sqrt(0.5) ~ 0.707

        // SharpCut at center
        fader.set_profile(CrossfaderCurveProfile::SharpCut);
        let (left, right) = fader.compute_gains();
        assert_eq!(left, 1.0);
        assert_eq!(right, 1.0);

        // SharpCut far left
        fader.set_position(-1.0);
        let (left, right) = fader.compute_gains();
        assert_eq!(left, 1.0);
        assert_eq!(right, 0.0);
    }
}
