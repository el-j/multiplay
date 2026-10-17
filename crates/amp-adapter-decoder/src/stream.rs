//! Stateful streaming audio decoder session for interactive track playback and seeking.

#![forbid(unsafe_code)]

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Stateful streaming audio decoder session for interactive track playback and seeking.
pub struct AudioStreamDecoder {
    reader: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    sample_buf: Option<SampleBuffer<f32>>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    duration_frames: Option<u64>,
    frames_played: u64,
    pending_samples: Vec<f32>,
    pending_idx: usize,
}

impl AudioStreamDecoder {
    /// Opens an audio file on disk and initializes format reader and codec decoder.
    pub fn open_file(path: &str) -> Result<Self, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open audio file {path}: {e}"))?;
        Self::open_media_source(file)
    }

    /// Opens an arbitrary MediaSource (File, Cursor, PrecacheBuffer).
    pub fn open_media_source<S>(source: S) -> Result<Self, String>
    where
        S: MediaSource + 'static,
    {
        let mss = MediaSourceStream::new(Box::new(source), Default::default());
        let hint = Hint::new();

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let meta_opts: MetadataOptions = Default::default();
        let decode_opts: DecoderOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &meta_opts)
            .map_err(|e| format!("Unrecognized audio format: {e}"))?;

        let reader = probed.format;
        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| "No playable audio tracks found".to_string())?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let duration_frames = track.codec_params.n_frames;
        let track_id = track.id;

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decode_opts)
            .map_err(|e| format!("Unsupported codec: {e}"))?;

        Ok(Self {
            reader,
            decoder,
            sample_buf: None,
            track_id,
            sample_rate,
            channels,
            duration_frames,
            frames_played: 0,
            pending_samples: Vec::new(),
            pending_idx: 0,
        })
    }

    /// Reads up to `output.len()` interleaved stereo f32 samples `[L, R, L, R, ...]`.
    /// Returns the number of samples actually written. Returns Ok(0) on EOF.
    pub fn read_interleaved_samples(&mut self, output: &mut [f32]) -> Result<usize, String> {
        let mut written = 0;

        // 1. Drain pending samples from previous packet
        while self.pending_idx < self.pending_samples.len() && written < output.len() {
            output[written] = self.pending_samples[self.pending_idx];
            written += 1;
            self.pending_idx += 1;
        }

        if self.pending_idx >= self.pending_samples.len() {
            self.pending_samples.clear();
            self.pending_idx = 0;
        }

        // 2. Decode next packets until buffer is filled or EOF
        while written < output.len() {
            let packet = match self.reader.next_packet() {
                Ok(pkt) => pkt,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break; // EOF
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => return Err(format!("Packet read error: {e}")),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(audio_buf) => {
                    if self.sample_buf.is_none() {
                        let audio_spec = *audio_buf.spec();
                        self.sample_buf = Some(SampleBuffer::new(audio_buf.capacity() as u64, audio_spec));
                    }

                    if let Some(buf) = self.sample_buf.as_mut() {
                        buf.copy_interleaved_ref(audio_buf);
                        let samples = buf.samples();

                        let channels = self.channels.max(1) as usize;
                        let frames = samples.len() / channels;

                        for f in 0..frames {
                            let (l, r) = downmix_frame(samples, f, channels);

                            if written + 1 < output.len() {
                                output[written] = l;
                                output[written + 1] = r;
                                written += 2;
                            } else {
                                self.pending_samples.push(l);
                                self.pending_samples.push(r);
                            }
                        }
                    }
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => return Err(format!("Decode frame error: {e}")),
            }
        }

        self.frames_played += (written / 2) as u64;
        Ok(written)
    }

    /// Seeks to specified playback duration timestamp.
    pub fn seek(&mut self, target: std::time::Duration) -> Result<(), String> {
        let ts = (target.as_secs_f64() * self.sample_rate as f64) as u64;
        let seek_to = symphonia::core::formats::SeekTo::TimeStamp {
            ts,
            track_id: self.track_id,
        };
        self.reader
            .seek(symphonia::core::formats::SeekMode::Coarse, seek_to)
            .map_err(|e| format!("Seek failed: {e}"))?;

        self.decoder.reset();
        self.pending_samples.clear();
        self.pending_idx = 0;
        self.frames_played = ts;
        Ok(())
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn duration(&self) -> Option<std::time::Duration> {
        self.duration_frames
            .map(|f| std::time::Duration::from_secs_f64(f as f64 / self.sample_rate as f64))
    }

    pub fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.frames_played as f64 / self.sample_rate as f64)
    }
}

/// Downmixes an interleaved multi-channel frame to stereo according to ITU-R BS.775.
pub fn downmix_frame(samples: &[f32], frame_idx: usize, channels: usize) -> (f32, f32) {
    let inv_sqrt_2 = std::f32::consts::FRAC_1_SQRT_2;
    let base = frame_idx * channels;
    if channels == 1 {
        (samples[frame_idx], samples[frame_idx])
    } else if channels == 2 {
        (samples[base], samples[base + 1])
    } else if channels >= 6 {
        let fl = samples[base];
        let fr = samples[base + 1];
        let fc = samples[base + 2];
        let sl = samples[base + 4];
        let sr = samples[base + 5];
        let left = (fl + fc * inv_sqrt_2 + sl * inv_sqrt_2) * inv_sqrt_2;
        let right = (fr + fc * inv_sqrt_2 + sr * inv_sqrt_2) * inv_sqrt_2;
        (left, right)
    } else {
        (samples[base], samples[base + 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downmix_mono_to_stereo() {
        let mono_samples = vec![0.8f32, 0.4f32];
        let (l0, r0) = downmix_frame(&mono_samples, 0, 1);
        assert_eq!(l0, 0.8);
        assert_eq!(r0, 0.8);

        let (l1, r1) = downmix_frame(&mono_samples, 1, 1);
        assert_eq!(l1, 0.4);
        assert_eq!(r1, 0.4);
    }

    #[test]
    fn test_downmix_stereo_passthrough() {
        let stereo = vec![0.7f32, -0.5f32];
        let (l, r) = downmix_frame(&stereo, 0, 2);
        assert_eq!(l, 0.7);
        assert_eq!(r, -0.5);
    }

    #[test]
    fn test_downmix_5_1_surround_sound() {
        // FL, FR, FC, LFE, SL, SR
        let surround = vec![1.0f32, 0.5f32, 1.0f32, 0.0f32, 0.5f32, 0.5f32];
        let (l, r) = downmix_frame(&surround, 0, 6);
        // Center channel and surround channels are mixed without phase error
        assert!(l > 0.0 && l <= 1.5);
        assert!(r > 0.0 && r <= 1.5);
    }
}
