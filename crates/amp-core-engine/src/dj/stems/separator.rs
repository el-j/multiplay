//! Offline stem separation for tracks and remix loop samples.

#![forbid(unsafe_code)]

use super::isolator::StemIsolator;
use super::params::StemType;

/// Four-stem separated audio buffers (interleaved stereo 32-bit float).
#[derive(Debug, Clone)]
pub struct SeparatedStems {
    /// Drums & rhythm section.
    pub drums: Vec<f32>,
    /// Bass & sub-bass.
    pub bass: Vec<f32>,
    /// Melodic synths, leads, and harmonic highs.
    pub melody: Vec<f32>,
    /// Vocals and lead presence.
    pub vocals: Vec<f32>,
    /// Audio sample rate.
    pub sample_rate: f32,
}

impl SeparatedStems {
    /// Separates interleaved stereo PCM audio into four phase-aligned stems.
    ///
    /// Separation runs allocation-free in the inner DSP loop, processing minutes
    /// of audio in tens of milliseconds.
    #[must_use]
    pub fn separate(interleaved_stereo: &[f32], sample_rate: f32) -> Self {
        let frame_count = interleaved_stereo.len() / 2;
        let mut drums = Vec::with_capacity(frame_count * 2);
        let mut bass = Vec::with_capacity(frame_count * 2);
        let mut melody = Vec::with_capacity(frame_count * 2);
        let mut vocals = Vec::with_capacity(frame_count * 2);

        let mut isolator = StemIsolator::new(sample_rate);

        for chunk in interleaved_stereo.as_chunks::<2>().0 {
            let l = chunk[0];
            let r = chunk[1];
            let stems = isolator.split_stems(l, r);

            // 0: Drums
            drums.push(stems[0][0]);
            drums.push(stems[0][1]);

            // 1: Bass
            bass.push(stems[1][0]);
            bass.push(stems[1][1]);

            // 2: Melody
            melody.push(stems[2][0]);
            melody.push(stems[2][1]);

            // 3: Vocals
            vocals.push(stems[3][0]);
            vocals.push(stems[3][1]);
        }

        Self {
            drums,
            bass,
            melody,
            vocals,
            sample_rate,
        }
    }

    /// Returns a slice of the interleaved stereo samples for the given stem.
    #[must_use]
    pub fn stem(&self, stem: StemType) -> &[f32] {
        match stem {
            StemType::Drums => &self.drums,
            StemType::Bass => &self.bass,
            StemType::Melody => &self.melody,
            StemType::Vocals => &self.vocals,
        }
    }
}
