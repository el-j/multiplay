//! Stateful streaming audio decoder session for interactive track playback and seeking.

#![forbid(unsafe_code)]

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Timestamp;

use crate::source::GenericMediaSource;

/// Stateful streaming audio decoder session for interactive track playback and seeking.
pub struct AudioStreamDecoder {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    /// The last decoded packet, interleaved in the stream's own channel layout.
    decoded: Vec<f32>,
    track_id: u32,
    sample_rate: u32,
    duration_frames: Option<u64>,
    frames_played: u64,
    pending_samples: Vec<f32>,
    pending_idx: usize,
    /// Frames still to drop after a seek landed before its target (sample-accurate seeking).
    skip_frames: u64,
}

impl AudioStreamDecoder {
    /// Opens an audio file on disk and initializes format reader and codec decoder.
    pub fn open_file(path: &str) -> Result<Self, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open audio file {path}: {e}"))?;
        Self::open_media_source(file)
    }

    /// Opens an arbitrary `Read + Seek + Send + Sync + 'static` stream (e.g. PrecacheBuffer, network stream, Cursor).
    pub fn open_stream<S>(stream: S) -> Result<Self, String>
    where
        S: std::io::Read + std::io::Seek + Send + Sync + 'static,
    {
        Self::open_media_source(GenericMediaSource::new(stream))
    }

    /// Opens an arbitrary MediaSource (File, Cursor, PrecacheBuffer).
    pub fn open_media_source<S>(source: S) -> Result<Self, String>
    where
        S: MediaSource + 'static,
    {
        let opened = open_audio(source)?;
        let (reader, decoder) = (opened.reader, opened.decoder);
        let (track_id, sample_rate, duration_frames) =
            (opened.track_id, opened.sample_rate, opened.duration_frames);

        Ok(Self {
            reader,
            decoder,
            decoded: Vec::new(),
            track_id,
            sample_rate,
            duration_frames,
            frames_played: 0,
            pending_samples: Vec::new(),
            pending_idx: 0,
            skip_frames: 0,
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

        let mut eof_reached = false;

        // 2. Decode next packets until buffer is filled or EOF
        while written < output.len() {
            let packet = match self.reader.next_packet() {
                Ok(Some(pkt)) => pkt,
                Ok(None) => {
                    eof_reached = true;
                    break; // End of the track
                }
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    eof_reached = true;
                    break; // Real EOF
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break; // Temporary network buffer underrun
                }
                Err(e) => return Err(format!("Packet read error: {e}")),
            };

            if packet.track_id != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(audio_buf) => {
                    let buf_channels = audio_buf.spec().channels().count().max(1);
                    let n = audio_buf.frames() * buf_channels;
                    self.decoded.resize(n, 0.0);
                    audio_buf.copy_to_slice_interleaved(&mut self.decoded[..]);
                    {
                        let samples = &self.decoded[..];

                        let channels = buf_channels;
                        let frames = samples.len() / channels;

                        for f in 0..frames {
                            if self.skip_frames > 0 {
                                self.skip_frames -= 1;
                                continue;
                            }
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

        if written == 0 && !eof_reached {
            return Err("WouldBlock".to_string());
        }

        self.frames_played += (written / 2) as u64;
        Ok(written)
    }

    /// Seeks to specified playback duration timestamp.
    pub fn seek(&mut self, target: std::time::Duration) -> Result<(), String> {
        let ts = (target.as_secs_f64() * self.sample_rate as f64).round() as u64;
        let seek_to = SeekTo::Timestamp {
            ts: Timestamp::new(ts as i64),
            track_id: self.track_id,
        };
        // The reader lands on a packet boundary at or before the target; the frames between
        // there and the target are decoded and dropped, so playback resumes on the exact frame.
        let landed = self
            .reader
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| format!("Seek failed: {e}"))?;
        self.skip_frames = (landed.required_ts.get() - landed.actual_ts.get()).max(0) as u64;

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

impl amp_ports::PcmStreamPort for AudioStreamDecoder {
    fn sample_rate(&self) -> u32 {
        AudioStreamDecoder::sample_rate(self)
    }

    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, String> {
        self.read_interleaved_samples(out)
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

/// A probed audio stream with its decoder.
pub(crate) struct OpenedAudio {
    pub reader: Box<dyn FormatReader>,
    pub decoder: Box<dyn AudioDecoder>,
    pub track_id: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_frames: Option<u64>,
}

/// Probes `source`, picks its first decodable audio track and makes a (gapless) decoder.
pub(crate) fn open_audio<S>(source: S) -> Result<OpenedAudio, String>
where
    S: MediaSource + 'static,
{
    let mss = MediaSourceStream::new(Box::new(source), Default::default());
    let reader = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("Unrecognized audio format: {e}"))?;
    let track = reader
        .first_track_known_codec(TrackType::Audio)
        .ok_or_else(|| "No playable audio tracks found".to_string())?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| "No playable audio tracks found".to_string())?
        .clone();
    let (track_id, duration_frames) = (track.id, track.num_frames);
    let decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| format!("Unsupported codec: {e}"))?;
    Ok(OpenedAudio {
        reader,
        decoder,
        track_id,
        sample_rate: params.sample_rate.unwrap_or(44_100),
        channels: params.channels.as_ref().map_or(2, |c| c.count() as u16),
        duration_frames,
    })
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
