//! Stateful streaming audio decoder session for interactive track playback and seeking.

#![forbid(unsafe_code)]

use amp_ports::{AudioSourceStream, DecodeError, DecoderFactoryPort, PcmStreamPort};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSource;
use symphonia::core::units::{TimeBase, Timestamp};

pub use crate::probe::build_probe_hint;
use crate::probe::{open_audio_with_hint, reject_unplayable};
use crate::source::GenericMediaSource;

/// Stateless factory implementing `DecoderFactoryPort` using Symphonia.
#[derive(Debug, Default, Clone, Copy)]
pub struct SymphoniaDecoderFactory;

impl DecoderFactoryPort for SymphoniaDecoderFactory {
    fn open_file(&self, path: &str) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if matches!(ext.as_deref(), Some("dsf" | "dff" | "wv")) {
            let file = std::fs::File::open(path).map_err(|e| {
                DecodeError::OpenFailed(format!("Failed to open audio file {path}: {e}"))
            })?;
            return self.open_stream(Box::new(file));
        }
        Ok(Box::new(AudioStreamDecoder::open_file(path)?))
    }

    fn open_stream(
        &self,
        mut stream: Box<dyn AudioSourceStream>,
    ) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
        // DSD (DSF/DSDIFF) and WavPack are not handled by Symphonia: recognise them by header.
        let mut head = [0u8; 4];
        let sniffed = stream.read_exact(&mut head).is_ok();
        stream
            .seek(std::io::SeekFrom::Start(0))
            .map_err(|e| DecodeError::OpenFailed(format!("Stream rewind failed: {e}")))?;
        if sniffed && crate::dsd::is_dsd(&head) {
            return Ok(Box::new(crate::dsd::DsdDecoder::open(stream)?));
        }
        if sniffed && crate::wavpack::is_wavpack(&head) {
            return Ok(Box::new(crate::wavpack::WavPackDecoder::open(stream)?));
        }
        Ok(Box::new(AudioStreamDecoder::open_stream(stream)?))
    }
}

/// Stateful streaming audio decoder session for interactive track playback and seeking.
pub struct AudioStreamDecoder {
    /// `None` only while [`Self::reopen_at_start`] swaps it.
    reader: Option<Box<dyn FormatReader>>,
    /// The probe hint, to open the stream again from its start.
    hint: Hint,
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
    /// Unit of the track's timestamps; `None` means one tick per sample frame.
    time_base: Option<TimeBase>,
    /// Target frame of the last seek until a packet arrives after it. Some readers report a
    /// seek as done but leave the stream at its end (Symphonia 0.6.1, ADPCM in WAV); then the
    /// decoder rewinds and decodes forward to this frame instead.
    seek_target: Option<u64>,
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
        let (reader, decoder) = (Some(opened.reader), opened.decoder);
        let (track_id, sample_rate, duration_frames, time_base) = (
            opened.track_id,
            opened.sample_rate,
            opened.duration_frames,
            opened.time_base,
        );

        Ok(Self {
            reader,
            hint,
            decoder,
            decoded: Vec::new(),
            track_id,
            sample_rate,
            duration_frames,
            time_base,
            seek_target: None,
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
            .as_ref()
            .ok_or_else(|| DecodeError::CorruptStream("the stream could not be reopened".into()))?
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
        self.time_base = track.time_base;
        if let Some(sr) = params.sample_rate {
            self.sample_rate = sr;
        }
        self.decoder = crate::probe::codec_registry()
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
            let packet = match self.reader()?.next_packet() {
                Ok(Some(pkt)) => pkt,
                Ok(None) => {
                    if let Some(target) = self.seek_target.take() {
                        self.rewind_and_skip_to(target)?;
                        continue;
                    }
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
                    // MP4 reports its end this way, also after a seek that "worked" once the
                    // end was reached: start over from the top, as for `Ok(None)` above.
                    if let Some(target) = self.seek_target.take() {
                        self.rewind_and_skip_to(target)?;
                        continue;
                    }
                    eof_reached = true;
                    break; // Real EOF
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break; // Temporary network buffer underrun
                }
                // Right after a seek, Symphonia 0.6.1's MP4 reader can fail with "no atom
                // pending read" once it had reached the end: start over, as above.
                Err(_) if self.seek_target.is_some() => {
                    let target = self.seek_target.take().unwrap_or_default();
                    self.rewind_and_skip_to(target)?;
                    continue;
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
            self.seek_target = None;

            // A damaged packet can make a codec panic (e.g. an overflow in MS-ADPCM): that fails
            // this file instead of the playback thread (TASK-185).
            let decoder = &mut self.decoder;
            let decoded = amp_ports::contain_panic(move || decoder.decode(&packet))
                .map_err(|e| DecodeError::CorruptStream(format!("damaged audio: {e}")))?;
            match decoded {
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
        // Timestamps are in the track's time base, which is not always one tick per sample
        // (Matroska/WebM count in their own ticks); rounding keeps sample-based seeks exact.
        let secs = target.as_secs_f64();
        let ticks = match self.time_base {
            Some(tb) => secs * tb.denom.get() as f64 / tb.numer.get() as f64,
            None => secs * self.sample_rate as f64,
        };
        let seek_to = SeekTo::Timestamp {
            ts: Timestamp::new(ticks.round() as i64),
            track_id: self.track_id,
        };
        // The reader lands on a packet boundary at or before the target; the frames between
        // there and the target are decoded and dropped, so playback resumes on the exact frame.
        let landed = match self.reader()?.seek(SeekMode::Accurate, seek_to) {
            Ok(landed) => landed,
            // Some readers refuse to seek once they have reached the end (Matroska in Symphonia
            // 0.6.1): open the stream again and decode forward to the target instead.
            Err(e) => {
                tracing::debug!("seek refused ({e}); reopening the stream");
                let frames = (secs * self.sample_rate as f64).round() as u64;
                self.rewind_and_skip_to(frames)?;
                self.decoder.reset();
                self.frames_played = frames;
                self.seek_target = None;
                return Ok(self.elapsed());
            }
        };
        let actual_ts = self.ts_to_frames(landed.actual_ts);
        let required_ts = self.ts_to_frames(landed.required_ts);
        self.skip_frames = required_ts.saturating_sub(actual_ts);

        self.decoder.reset();
        self.pending_samples.clear();
        self.pending_idx = 0;
        let effective_frames = actual_ts + self.skip_frames;
        self.frames_played = effective_frames;
        self.seek_target = Some(required_ts);
        Ok(self.elapsed())
    }

    fn reader(&mut self) -> Result<&mut Box<dyn FormatReader>, DecodeError> {
        self.reader
            .as_mut()
            .ok_or_else(|| DecodeError::CorruptStream("the stream could not be reopened".into()))
    }

    /// Fallback for a reader whose seek left it at the end (ADPCM in WAV, and MP4 once it has
    /// reached its end; Symphonia 0.6.1): open the stream again from its start and drop frames
    /// up to `target`. A fresh reader and decoder, so no state of the stuck one survives.
    fn rewind_and_skip_to(&mut self, target: u64) -> Result<(), DecodeError> {
        self.reopen_at_start()?;
        self.skip_frames = target;
        Ok(())
    }

    fn reopen_at_start(&mut self) -> Result<(), DecodeError> {
        let reader = self
            .reader
            .take()
            .ok_or_else(|| DecodeError::CorruptStream("the stream could not be reopened".into()))?;
        let mut source = reader.into_inner();
        std::io::Seek::seek(&mut source, std::io::SeekFrom::Start(0))
            .map_err(|e| DecodeError::SeekFailed(format!("Rewind: {e}")))?;
        let opened = open_audio_with_hint(source, &self.hint).map_err(DecodeError::SeekFailed)?;
        self.reader = Some(opened.reader);
        self.decoder = opened.decoder;
        self.track_id = opened.track_id;
        self.pending_samples.clear();
        self.pending_idx = 0;
        Ok(())
    }

    /// Sample frames for a timestamp in this track's time base.
    fn ts_to_frames(&self, ts: Timestamp) -> u64 {
        let ticks = ts.get().max(0) as u64;
        match self.time_base {
            Some(tb) => tb.calc_time(ts).map_or(ticks, |t| {
                (t.as_secs_f64().max(0.0) * self.sample_rate as f64).round() as u64
            }),
            None => ticks,
        }
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
        // Whole stereo pairs only: half a pair would start the next read on the right channel
        // and swap the channels from then on (found by the port contract, TASK-187).
        let pairs = out.len() / 2 * 2;
        let n = self.read_interleaved_samples(&mut out[..pairs])?;
        sanitize(&mut out[..n]);
        Ok(n)
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

/// Loudest sample let through: room for inter-sample overs in float files, which the limiter
/// catches, but not for the huge values a damaged float file decodes to.
const MAX_SAMPLE: f32 = 2.0;

/// Damaged float audio can decode to NaN, infinity or huge values. NaN would stick in the EQ's
/// filters and silence the rest of the track, and huge values would be painfully loud, so they
/// become silence and the loudest allowed sample here (TASK-185).
fn sanitize(samples: &mut [f32]) {
    for s in samples {
        if !s.is_finite() {
            *s = 0.0;
        } else if s.abs() > MAX_SAMPLE {
            *s = s.signum() * MAX_SAMPLE;
        }
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
            (Some("mpc"), None, "Musepack"),
            (Some(".TTA"), None, "TTA"),
            (Some("ape"), None, "Monkey's Audio"),
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
    fn test_downmix_mono_stereo_and_5_1_surround() {
        assert_eq!(downmix_frame(&[0.8f32, 0.4f32], 0, 1), (0.8, 0.8));
        assert_eq!(downmix_frame(&[0.8f32, 0.4f32], 1, 1), (0.4, 0.4));
        assert_eq!(downmix_frame(&[0.7f32, -0.5f32], 0, 2), (0.7, -0.5));
        let (l, r) = downmix_frame(&[1.0f32, 0.5, 1.0, 0.0, 0.5, 0.5], 0, 6);
        assert!(l > 0.0 && l <= 1.5 && r > 0.0 && r <= 1.5);
    }
}
