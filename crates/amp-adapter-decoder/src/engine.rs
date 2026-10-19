//! Stateless pure decoder engine writing directly into a lock-free ring buffer producer.

#![forbid(unsafe_code)]

use rtrb::Producer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::io::MediaSource;

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
        let opened = crate::stream::open_audio(stream)?;
        let (mut reader, mut decoder, track_id) = (opened.reader, opened.decoder, opened.track_id);
        let spec = DecodedStreamSpec {
            sample_rate: opened.sample_rate,
            channels: opened.channels,
            duration_frames: opened.duration_frames,
        };
        let mut samples: Vec<f32> = Vec::new();

        while !stop_signal.load(Ordering::Relaxed) {
            let packet = match reader.next_packet() {
                Ok(Some(pkt)) => pkt,
                Ok(None) => break,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Buffer underrun in precaching stream: yield briefly and retry
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Err(e) => return Err(format!("Demux packet error: {e}")),
            };

            if packet.track_id != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    samples.resize(
                        audio_buf.frames() * audio_buf.spec().channels().count(),
                        0.0,
                    );
                    audio_buf.copy_to_slice_interleaved(&mut samples[..]);
                    for &sample in &samples {
                        // Backpressure handling: yield thread if ring buffer is full
                        while producer.push(sample).is_err() {
                            if stop_signal.load(Ordering::Relaxed) {
                                return Ok(spec);
                            }
                            std::thread::yield_now();
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
