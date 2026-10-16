//! Stateless pure decoder engine writing directly into a lock-free ring buffer producer.

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

/// Specification metadata of a decoded audio stream.
#[derive(Debug, Clone)]
pub struct DecodedStreamSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_frames: Option<u64>,
}

/// Pure decoder engine streaming decoded samples directly into an SPSC ring buffer.
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
