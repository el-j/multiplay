//! Stateful streaming audio decoder session for interactive track playback and seeking.

#![forbid(unsafe_code)]

use amp_ports::{AudioSourceStream, DecodeError, DecoderFactoryPort, PcmStreamPort};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSource;
use symphonia::core::units::Timestamp;

use crate::probe::{open_audio_with_hint, reject_unplayable};
pub use crate::probe::build_probe_hint;
use crate::source::GenericMediaSource;

/// Stateless factory implementing `DecoderFactoryPort` using Symphonia.
#[derive(Debug, Default, Clone, Copy)]
pub struct SymphoniaDecoderFactory;

impl DecoderFactoryPort for SymphoniaDecoderFactory {
    fn open_file(&self, path: &str) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
        Ok(Box::new(AudioStreamDecoder::open_file(path)?))
    }

    fn open_stream(
        &self,
        stream: Box<dyn AudioSourceStream>,
    ) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
        Ok(Box::new(AudioStreamDecoder::open_stream(stream)?))
    }
}

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
    /// Opens an audio file on disk and initializes format reader and codec decoder,
    /// passing the file extension as a Symphonia probe [`Hint`] (`TASK-139`).
    pub fn open_file(path: &str) -> Result<Self, DecodeError> {
        let file = std::fs::File::open(path).map_err(|e| {
            DecodeError::OpenFailed(format!("Failed to open audio file {path}: {e}"))
        })?;
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str());
        reject_unplayable(ext, None)?;
        let hint = build_probe_hint(ext, None);
        Self::open_media_source_with_hint(file, hint)
    }

    /// Opens an arbitrary `Read + Seek + Send + Sync + 'static` stream (e.g. PrecacheBuffer, network stream, Cursor).
    pub fn open_stream<S>(stream: S) -> Result<Self, DecodeError>
    where
        S: std::io::Read + std::io::Seek + Send + Sync + 'static,
    {
        Self::open_media_source(GenericMediaSource::new(stream))
    }

    /// Opens an arbitrary stream with explicit file-extension and/or MIME type probe hints (`TASK-139`).
    pub fn open_stream_with_hint<S>(
        stream: S,
        extension: Option<&str>,
        mime_type: Option<&str>,
    ) -> Result<Self, DecodeError>
    where
        S: std::io::Read + std::io::Seek + Send + Sync + 'static,
    {
        reject_unplayable(extension, mime_type)?;
        let hint = build_probe_hint(extension, mime_type);
        Self::open_media_source_with_hint(GenericMediaSource::new(stream), hint)
    }

    /// Opens an arbitrary MediaSource (File, Cursor, PrecacheBuffer).
    pub fn open_media_source<S>(source: S) -> Result<Self, DecodeError>
    where
        S: MediaSource + 'static,
    {
        Self::open_media_source_with_hint(source, Hint::new())
    }

    /// Opens an arbitrary MediaSource with a Symphonia probe [`Hint`].
    pub fn open_media_source_with_hint<S>(source: S, hint: Hint) -> Result<Self, DecodeError>
    where
        S: MediaSource + 'static,
    {
        let opened = open_audio_with_hint(source, &hint).map_err(DecodeError::UnsupportedFormat)?;
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

    /// Rebuilds the active codec decoder when a chained stream (e.g. chained Ogg Vorbis)
    /// signals `SymphoniaError::ResetRequired` (`TASK-139`).
    pub fn rebuild_decoder_on_reset(&mut self) -> Result<(), DecodeError> {
        let track = self
            .reader
            .first_track_known_codec(TrackType::Audio)
            .ok_or_else(|| {
                DecodeError::UnsupportedFormat(
                    "No playable audio track after stream reset".to_string(),
                )
            })?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or_else(|| {
                DecodeError::UnsupportedFormat(
                    "Missing audio codec parameters after stream reset".to_string(),
                )
            })?
            .clone();
        self.track_id = track.id;
        if let Some(sr) = params.sample_rate {
            self.sample_rate = sr;
        }
        self.decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .map_err(|e| DecodeError::UnsupportedFormat(format!("Unsupported codec: {e}")))?;
        Ok(())
    }

    /// Reads up to `output.len()` interleaved stereo f32 samples `[L, R, L, R, ...]`.
    /// Returns the number of samples actually written. Returns Ok(0) on EOF.
    pub fn read_interleaved_samples(&mut self, output: &mut [f32]) -> Result<usize, DecodeError> {
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
                Err(SymphoniaError::ResetRequired) => {
                    self.rebuild_decoder_on_reset()?;
                    continue;
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
                Err(e) => {
                    return Err(DecodeError::CorruptStream(format!(
                        "Packet read error: {e}"
                    )))
                }
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
                Err(SymphoniaError::ResetRequired) => {
                    self.rebuild_decoder_on_reset()?;
                    continue;
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => {
                    return Err(DecodeError::CorruptStream(format!(
                        "Decode frame error: {e}"
                    )))
                }
            }
        }

        if written == 0 && !eof_reached {
            return Err(DecodeError::WouldBlock);
        }

        self.frames_played += (written / 2) as u64;
        Ok(written)
    }

    /// Seeks to specified playback duration timestamp and returns the actual seeked timestamp.
    pub fn seek(
        &mut self,
        target: std::time::Duration,
    ) -> Result<std::time::Duration, DecodeError> {
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
            .map_err(|e| DecodeError::SeekFailed(format!("Seek failed: {e}")))?;
        let actual_ts = landed.actual_ts.get().max(0) as u64;
        let required_ts = landed.required_ts.get().max(0) as u64;
        self.skip_frames = required_ts.saturating_sub(actual_ts);

        self.decoder.reset();
        self.pending_samples.clear();
        self.pending_idx = 0;
        let effective_frames = actual_ts + self.skip_frames;
        self.frames_played = effective_frames;
        Ok(self.elapsed())
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

impl PcmStreamPort for AudioStreamDecoder {
    fn sample_rate(&self) -> u32 {
        AudioStreamDecoder::sample_rate(self)
    }

    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, DecodeError> {
        self.read_interleaved_samples(out)
    }

    fn duration(&self) -> Option<std::time::Duration> {
        AudioStreamDecoder::duration(self)
    }

    fn elapsed(&self) -> std::time::Duration {
        AudioStreamDecoder::elapsed(self)
    }

    fn seek(&mut self, target: std::time::Duration) -> Result<std::time::Duration, DecodeError> {
        AudioStreamDecoder::seek(self, target)
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
    use std::io::Cursor;
    use std::time::Duration;

    fn build_ramp_wav(num_frames: usize, sample_rate: u32) -> Vec<u8> {
        let data_size = (num_frames * 4) as u32;
        let mut bytes = Vec::with_capacity(44 + data_size as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt \x10\x00\x00\x00\x01\x00\x02\x00");
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 4).to_le_bytes());
        bytes.extend_from_slice(b"\x04\x00\x10\x00data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for f in 0..num_frames {
            let sample = (f as i16).clamp(-30_000, 30_000);
            bytes.extend_from_slice(&sample.to_le_bytes());
            bytes.extend_from_slice(&(-sample).to_le_bytes());
        }
        bytes
    }

    #[test]
    fn seek_accuracy_within_one_frame_on_wav() {
        let sample_rate = 10_000;
        let wav = build_ramp_wav(20_000, sample_rate);
        let mut dec = AudioStreamDecoder::open_stream_with_hint(
            Cursor::new(wav),
            Some("wav"),
            Some("audio/wav"),
        )
        .unwrap();

        for &target_frame in &[500usize, 2_500, 7_777, 12_345, 1_000] {
            let target_dur = Duration::from_secs_f64(target_frame as f64 / sample_rate as f64);
            dec.seek(target_dur).unwrap();
            let mut out = [0.0f32; 2];
            assert_eq!(dec.read_interleaved_samples(&mut out).unwrap(), 2);
            let actual_frame = (out[0] * 32768.0).round() as isize;
            assert!((actual_frame - target_frame as isize).abs() <= 1);
        }
    }

    #[test]
    fn reset_required_rebuilds_decoder_seamlessly() {
        let wav = build_ramp_wav(1024, 44_100);
        let mut dec = AudioStreamDecoder::open_stream(Cursor::new(wav)).unwrap();
        let mut out = [0.0f32; 64];
        assert_eq!(dec.read_interleaved_samples(&mut out).unwrap(), 64);
        dec.rebuild_decoder_on_reset().unwrap();
        assert_eq!(dec.read_interleaved_samples(&mut out).unwrap(), 64);
    }

    #[test]
    fn formats_the_registry_cannot_play_are_rejected_by_name() {
        for (ext, mime, name) in [
            (Some("wv"), None, "WavPack"),
            (None, Some("audio/opus"), "Opus"),
            (Some(".OPUS"), None, "Opus"),
        ] {
            match AudioStreamDecoder::open_stream_with_hint(Cursor::new(vec![0u8; 64]), ext, mime) {
                Err(DecodeError::UnsupportedFormat(msg)) => assert!(msg.contains(name), "{msg}"),
                other => panic!(
                    "{ext:?}/{mime:?}: expected UnsupportedFormat, got {:?}",
                    other.err()
                ),
            }
        }
    }

    #[test]
    fn opus_is_cleanly_rejected_with_unsupported_format() {
        let res = AudioStreamDecoder::open_stream_with_hint(
            Cursor::new(vec![0u8; 64]),
            Some("opus"),
            Some("audio/opus"),
        );
        assert!(matches!(res, Err(DecodeError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_downmix_mono_stereo_and_5_1_surround() {
        assert_eq!(downmix_frame(&[0.8f32, 0.4f32], 0, 1), (0.8, 0.8));
        assert_eq!(downmix_frame(&[0.8f32, 0.4f32], 1, 1), (0.4, 0.4));
        assert_eq!(downmix_frame(&[0.7f32, -0.5f32], 0, 2), (0.7, -0.5));
        let (l, r) = downmix_frame(&[1.0f32, 0.5, 1.0, 0.0, 0.5, 0.5], 0, 6);
        assert!(l > 0.0 && l <= 1.5 && r > 0.0 && r <= 1.5);
    }
}
