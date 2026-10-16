//! Pure-Rust multi-format audio decoder engine using Symphonia.
//! Decodes MP3, FLAC, WAV, AAC, Vorbis, and Opus into interleaved normalized f32 PCM samples.

#![forbid(unsafe_code)]

use rtrb::Producer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, Clone)]
pub struct DecodedStreamSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_frames: Option<u64>,
}

pub struct PureDecoderEngine;

impl PureDecoderEngine {
    /// Decodes an audio stream directly into a lock-free RTRB ring buffer producer.
    pub fn decode_stream_to_ring_buffer<S>(
        stream: S,
        producer: &mut Producer<f32>,
        stop_signal: Arc<AtomicBool>,
    ) -> Result<DecodedStreamSpec, String>
    where
        S: MediaSource + 'static,
    {
        let mss = MediaSourceStream::new(Box::new(stream), Default::default());
        let hint = Hint::new();

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let meta_opts: MetadataOptions = Default::default();
        let decode_opts: DecoderOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &meta_opts)
            .map_err(|e| format!("Unrecognized audio container: {e}"))?;

        let mut reader = probed.format;
        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| "No playable audio tracks found".to_string())?;

        let spec = DecodedStreamSpec {
            sample_rate: track.codec_params.sample_rate.unwrap_or(44100),
            channels: track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2),
            duration_frames: track.codec_params.n_frames,
        };

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decode_opts)
            .map_err(|e| format!("Unsupported audio codec: {e}"))?;

        let track_id = track.id;
        let mut sample_buf: Option<SampleBuffer<f32>> = None;

        while !stop_signal.load(Ordering::Relaxed) {
            let packet = match reader.next_packet() {
                Ok(pkt) => pkt,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Buffer underrun in precaching stream: yield briefly and retry
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Err(e) => return Err(format!("Demux packet error: {e}")),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    if sample_buf.is_none() {
                        let audio_spec = *audio_buf.spec();
                        sample_buf = Some(SampleBuffer::new(audio_buf.capacity() as u64, audio_spec));
                    }

                    if let Some(buf) = sample_buf.as_mut() {
                        buf.copy_interleaved_ref(audio_buf);
                        for &sample in buf.samples() {
                            // Backpressure handling: yield thread if ring buffer is full
                            while producer.push(sample).is_err() {
                                if stop_signal.load(Ordering::Relaxed) {
                                    return Ok(spec);
                                }
                                std::thread::yield_now();
                            }
                        }
                    }
                }
                Err(SymphoniaError::DecodeError(msg)) => {
                    eprintln!("Recoverable decode frame warning: {msg}");
                    continue;
                }
                Err(e) => return Err(format!("Fatal decoding failure: {e}")),
            }
        }

        Ok(spec)
    }
}

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
                            let (l, r) = if channels == 1 {
                                (samples[f], samples[f])
                            } else {
                                (samples[f * channels], samples[f * channels + 1])
                            };

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

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;
    use std::io::Cursor;
    use std::time::Duration;

    /// Helper to generate a minimal valid 16-bit PCM WAV in memory
    fn generate_test_wav_bytes(num_samples: usize, sample_rate: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let data_size = (num_samples * 2 * 2) as u32; // 2 channels, 16-bit (2 bytes per sample)
        let file_size = 36 + data_size;

        // RIFF header
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        // fmt chunk
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
        bytes.extend_from_slice(&1u16.to_le_bytes());  // PCM format
        bytes.extend_from_slice(&2u16.to_le_bytes());  // Stereo 2 channels
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * 2 * 2;
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());  // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        // data chunk
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());

        // Fill with synthetic sine wave PCM
        for i in 0..num_samples {
            let val = ((i as f32 * 0.1).sin() * 32767.0) as i16;
            bytes.extend_from_slice(&val.to_le_bytes()); // Left
            bytes.extend_from_slice(&val.to_le_bytes()); // Right
        }

        bytes
    }

    #[test]
    fn test_decode_wav_to_ring_buffer() {
        let wav_data = generate_test_wav_bytes(256, 44100);
        let cursor = Cursor::new(wav_data);

        let (mut producer, mut consumer) = RingBuffer::<f32>::new(4096);
        let stop_signal = Arc::new(AtomicBool::new(false));

        let spec = PureDecoderEngine::decode_stream_to_ring_buffer(
            cursor,
            &mut producer,
            stop_signal,
        )
        .expect("decoding test wav failed");

        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.channels, 2);

        // Verify samples arrived in the consumer
        let mut sample_count = 0;
        while let Ok(sample) = consumer.pop() {
            assert!((-1.0..=1.0).contains(&sample));
            sample_count += 1;
        }
        assert_eq!(sample_count, 256 * 2); // 256 frames * 2 channels
    }

    #[test]
    fn test_audio_stream_decoder_read_and_seek() {
        let wav_data = generate_test_wav_bytes(512, 44100);
        let cursor = Cursor::new(wav_data);

        let mut decoder = AudioStreamDecoder::open_media_source(cursor)
            .expect("Failed to open audio stream decoder on wav");

        assert_eq!(decoder.sample_rate(), 44100);

        let mut buffer = vec![0.0f32; 256]; // 128 frames
        let read = decoder.read_interleaved_samples(&mut buffer).expect("read failed");
        assert_eq!(read, 256);
        assert!((decoder.elapsed().as_secs_f64() - 128.0 / 44100.0).abs() < 1e-5);

        // Test seek
        decoder.seek(Duration::from_millis(5)).expect("seek failed");
        assert!(decoder.elapsed() <= Duration::from_millis(10));
    }
}

