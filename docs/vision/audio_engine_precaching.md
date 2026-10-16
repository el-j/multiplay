# Real-Time Audio Engine, Buffer Precaching & Output

This document contains the low-latency audio processing loop, RAM precaching mechanics, Symphonia-to-CPAL bridging, and the lock-free ring-buffer pipeline.

---

## 1. High-Performance Audio Thread Topology

```
                  ┌────────────────────────────────────────────────────────┐
                  │                 Network / Disk Reader                  │
                  │              (Async Tokio Worker Task)                 │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                  Bytes via append_bytes()
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │            MemoryPrecacheBuffer (64 MB RAM)            │
                  │        Implements std::io::Read + std::io::Seek        │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                   Decoded via Symphonia
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │              Symphonia Decoding Pipeline               │
                  │            Outputs raw interleaved &[f32]              │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                 rtrb Producer (Lock-Free)
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │             rtrb Ring Buffer (SPSC Queue)              │
                  │                  Capacity: 65,536 frames               │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                 rtrb Consumer (Lock-Free)
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │               CPAL Real-Time Audio Loop                │
                  │               (Direct macOS CoreAudio)                 │
                  └────────────────────────────────────────────────────────┘
```

---

## 2. In-Memory Precaching Stream (`amp-core-engine/src/precache.rs`)

This stream allows Symphonia to read and seek instantly across cached bytes while background network tasks asynchronously push incoming packets into the same container.

```rust
// crates/amp-core-engine/src/precache.rs
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct SharedPrecacheBuffer {
    inner: Arc<RwLock<PrecacheBufferState>>,
}

struct PrecacheBufferState {
    data: Vec<u8>,
    cursor: usize,
    stream_ended: bool,
}

impl SharedPrecacheBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(PrecacheBufferState {
                data: Vec::with_capacity(capacity),
                cursor: 0,
                stream_ended: false,
            })),
        }
    }

    pub fn push_chunk(&self, chunk: &[u8]) {
        let mut state = self.inner.write().unwrap();
        state.data.extend_from_slice(chunk);
    }

    pub fn mark_finished(&self) {
        let mut state = self.inner.write().unwrap();
        state.stream_ended = true;
    }

    pub fn total_bytes_cached(&self) -> usize {
        self.inner.read().unwrap().data.len()
    }
}

impl Read for SharedPrecacheBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut state = self.inner.write().unwrap();
        let available = state.data.len().saturating_sub(state.cursor);

        if available == 0 {
            if state.stream_ended {
                return Ok(0); // Clean EOF
            }
            // Signal decoder to yield momentarily while network fills
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Precache underflow: awaiting incoming network chunks",
            ));
        }

        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&state.data[state.cursor..state.cursor + to_read]);
        state.cursor += to_read;
        Ok(to_read)
    }
}

impl Seek for SharedPrecacheBuffer {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mut state = self.inner.write().unwrap();
        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => state.cursor as i64 + offset,
            SeekFrom::End(offset) => state.data.len() as i64 + offset,
        };

        if target < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid seek to negative position"));
        }

        state.cursor = (target as usize).min(state.data.len());
        Ok(state.cursor as u64)
    }
}
```

---

## 3. Pure-Rust CPAL Hardware Sink (`amp-adapter-cpal/src/lib.rs`)

```rust
// crates/amp-adapter-cpal/src/lib.rs
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use rtrb::{Consumer, RingBuffer};
use std::sync::Arc;

pub struct CpalAudioSink {
    device: cpal::Device,
    config: StreamConfig,
    active_stream: Option<Stream>,
    ring_consumer: Option<Consumer<f32>>,
}

impl CpalAudioSink {
    pub fn new() -> Result<(Self, rtrb::Producer<f32>), String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default audio output device found".to_string())?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| format!("Failed to read device config: {e}"))?;

        let sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        let config = StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (producer, consumer) = RingBuffer::<f32>::new(65536);

        let sink = Self {
            device,
            config,
            active_stream: None,
            ring_consumer: Some(consumer),
        };

        Ok((sink, producer))
    }

    pub fn start_playback(&mut self) -> Result<(), String> {
        if self.ring_consumer.is_none() {
            return Err("Consumer already bound to stream".to_string());
        }

        let mut consumer = self.ring_consumer.take().unwrap();

        let error_callback = |err| eprintln!("Audio stream callback error: {err}");

        let data_callback = move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for sample in output.iter_mut() {
                *sample = consumer.pop().unwrap_or(0.0);
            }
        };

        let stream = self
            .device
            .build_output_stream(&self.config, data_callback, error_callback, None)
            .map_err(|e| format!("Stream building failed: {e}"))?;

        stream.play().map_err(|e| format!("Playback launch failed: {e}"))?;
        self.active_stream = Some(stream);
        Ok(())
    }
}
```

---

## 4. Symphonia Decoding Engine (`amp-adapter-decoder/src/lib.rs`)

```rust
// crates/amp-adapter-decoder/src/lib.rs
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use std::io::{Read, Seek};

pub struct PureDecoderEngine;

impl PureDecoderEngine {
    pub fn decode_stream_to_ring_buffer<S>(
        stream: S,
        producer: &mut rtrb::Producer<f32>,
        stop_signal: std::sync::atomic::AtomicBool,
    ) -> Result<(), String>
    where
        S: Read + Seek + Send + Sync + 'static,
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

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decode_opts)
            .map_err(|e| format!("Unsupported audio codec: {e}"))?;

        let track_id = track.id;
        let mut sample_buf: Option<SampleBuffer<f32>> = None;

        while !stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
            let packet = match reader.next_packet() {
                Ok(pkt) => pkt,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(format!("Demux packet error: {e}")),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    if sample_buf.is_none() {
                        let spec = *audio_buf.spec();
                        sample_buf = Some(SampleBuffer::new(audio_buf.capacity() as u64, spec));
                    }

                    if let Some(buf) = sample_buf.as_mut() {
                        buf.copy_interleaved_ref(audio_buf);
                        for &sample in buf.samples() {
                            // Backpressure: Spin or yield if hardware ring buffer is full
                            while producer.push(sample).is_err() {
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
        Ok(())
    }
}
```