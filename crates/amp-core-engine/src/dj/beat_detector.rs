//! Pure-Rust real-time BPM detection and transient onset estimator.
//! Operates on energy flux with dynamic variance thresholding and refractory blanking.

#![forbid(unsafe_code)]

/// History buffer size for rolling audio block energies (~1 second of audio at 1024-sample blocks).
const ENERGY_HISTORY_LEN: usize = 43;

/// Pure-Rust real-time BPM detector and beat-grid phase tracker.
#[derive(Debug, Clone)]
pub struct RealtimeBpmDetector {
    sample_rate: f32,
    block_size: usize,
    energy_history: [f32; ENERGY_HISTORY_LEN],
    history_idx: usize,
    history_filled: usize,
    samples_since_last_beat: usize,
    min_beat_interval_samples: usize,
    ibi_history: [f32; 8],
    ibi_count: usize,
    estimated_bpm: f32,
}

impl RealtimeBpmDetector {
    /// Creates a new detector instance configured for a specific audio sample rate and block size.
    #[must_use]
    pub fn new(sample_rate: f32, block_size: usize) -> Self {
        // Enforce refractory period corresponding to ~220 BPM max (min ~272 ms)
        let min_beat_interval = (sample_rate * 0.27) as usize;

        Self {
            sample_rate,
            block_size,
            energy_history: [0.0; ENERGY_HISTORY_LEN],
            history_idx: 0,
            history_filled: 0,
            samples_since_last_beat: min_beat_interval,
            min_beat_interval_samples: min_beat_interval,
            ibi_history: [0.5; 8], // 0.5s default -> 120 BPM
            ibi_count: 0,
            estimated_bpm: 120.0,
        }
    }

    /// Configured block size for energy analysis.
    #[must_use]
    pub const fn block_size(&self) -> usize {
        self.block_size
    }

    /// Current smoothed tempo estimation in beats per minute.
    #[must_use]
    pub const fn estimated_bpm(&self) -> f32 {
        self.estimated_bpm
    }

    /// Current phase offset within the beat cycle (0.0 to 1.0).
    #[must_use]
    pub fn phase(&self) -> f32 {
        let beat_duration_samples = (self.sample_rate * 60.0 / self.estimated_bpm).max(1.0);
        let current_pos = (self.samples_since_last_beat as f32) % beat_duration_samples;
        current_pos / beat_duration_samples
    }

    /// Processes an audio frame block and returns `true` if a transient beat onset was detected.
    pub fn process_block(&mut self, samples: &[f32]) -> bool {
        if samples.is_empty() {
            return false;
        }

        self.samples_since_last_beat += samples.len();

        // 1. Calculate instant block energy: sum(s^2)
        let mut instant_energy = 0.0_f32;
        for &s in samples {
            instant_energy += s * s;
        }

        // 2. Compute local average historical energy
        let count = self.history_filled.max(1);
        let sum_energy: f32 = self.energy_history.iter().take(count).sum();
        let avg_energy = sum_energy / (count as f32);

        // 3. Compute energy variance
        let mut variance = 0.0_f32;
        for &e in self.energy_history.iter().take(count) {
            let diff = e - avg_energy;
            variance += diff * diff;
        }
        variance /= count as f32;

        // 4. Dynamic threshold constant C based on variance
        let c = (-0.00257 * variance).clamp(-0.5, 0.5) + 1.35;
        let threshold = c * avg_energy;

        // 5. Update circular history buffer
        self.energy_history[self.history_idx] = instant_energy;
        self.history_idx = (self.history_idx + 1) % ENERGY_HISTORY_LEN;
        if self.history_filled < ENERGY_HISTORY_LEN {
            self.history_filled += 1;
        }

        // 6. Check for onset with refractory period
        if instant_energy > threshold
            && instant_energy > 0.01
            && self.samples_since_last_beat >= self.min_beat_interval_samples
        {
            let interval_secs = self.samples_since_last_beat as f32 / self.sample_rate;
            self.samples_since_last_beat = 0;

            if interval_secs > 0.25 && interval_secs < 1.5 {
                let slot = self.ibi_count % self.ibi_history.len();
                self.ibi_history[slot] = interval_secs;
                self.ibi_count += 1;

                let valid_count = self.ibi_count.min(self.ibi_history.len());
                let avg_ibi: f32 =
                    self.ibi_history.iter().take(valid_count).sum::<f32>() / (valid_count as f32);

                if avg_ibi > 0.0 {
                    let instant_bpm = 60.0 / avg_ibi;
                    // Low-pass smooth BPM
                    self.estimated_bpm =
                        (self.estimated_bpm * 0.7 + instant_bpm * 0.3).clamp(60.0, 200.0);
                }
            }

            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpm_detector_synthetic_pulses() {
        let sample_rate = 44100.0;
        let block_size = 512;
        let mut detector = RealtimeBpmDetector::new(sample_rate, block_size);

        // Simulate 120 BPM: 0.5 seconds between beats = 22050 samples = 43 blocks of silence then 1 loud block
        let blocks_between_beats = (sample_rate * 0.5 / block_size as f32).round() as usize;

        let silent_block = vec![0.001_f32; block_size];
        let mut pulse_block = vec![0.001_f32; block_size];
        for s in pulse_block.iter_mut().take(64) {
            *s = 0.9;
        }

        let mut beat_count = 0;
        for i in 0..(blocks_between_beats * 12) {
            let is_beat = if i % blocks_between_beats == 0 {
                detector.process_block(&pulse_block)
            } else {
                detector.process_block(&silent_block)
            };

            if is_beat {
                beat_count += 1;
            }
        }

        assert!(
            beat_count >= 5,
            "Should have detected at least 5 pulses, found: {}",
            beat_count
        );
        let bpm = detector.estimated_bpm();
        assert!(
            (bpm - 120.0).abs() < 15.0,
            "BPM should converge near 120, got: {}",
            bpm
        );
    }
}
