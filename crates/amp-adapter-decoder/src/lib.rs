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

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;
    use std::io::Cursor;

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
            assert!(sample >= -1.0 && sample <= 1.0);
            sample_count += 1;
        }
        assert_eq!(sample_count, 256 * 2); // 256 frames * 2 channels
    }
}
