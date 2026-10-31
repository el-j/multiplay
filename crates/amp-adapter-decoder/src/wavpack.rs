//! WavPack playback (TASK-231) with the pure-Rust `wavicle` codec.
//!
//! `wavicle` decodes whole streams; every WavPack block is self-contained, so this adapter walks
//! the block headers itself and decodes one block at a time (memory stays at one block), stops at
//! the APEv2 tag most files end with, and seeks by skipping whole blocks.

#![forbid(unsafe_code)]

use amp_ports::{AudioSourceStream, DecodeError, PcmStreamPort};
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

const HEADER: usize = 32;
const FLAG_INITIAL: u32 = 0x800;
const FLAG_FINAL: u32 = 0x1000;
/// Largest block accepted (the format allows 1 MB).
const MAX_BLOCK: usize = 1 << 20;

/// Whether `head` starts a WavPack block.
pub fn is_wavpack(head: &[u8]) -> bool {
    head.starts_with(b"wvpk")
}

fn err(msg: impl Into<String>) -> DecodeError {
    DecodeError::UnsupportedFormat(format!("WavPack: {}", msg.into()))
}

/// The fields of a block header this adapter needs.
#[derive(Debug, Clone, Copy)]
struct Header {
    /// Whole block length including the header.
    len: usize,
    /// First sample of the block (per channel).
    index: u64,
    samples: u32,
    flags: u32,
    /// Samples in the whole file, if the header knows.
    total: Option<u64>,
}

fn parse_header(h: &[u8; HEADER]) -> Option<Header> {
    if !is_wavpack(h) {
        return None;
    }
    let u32_at = |i: usize| u32::from_le_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
    let total_low = u32_at(12);
    Some(Header {
        len: u32_at(4) as usize + 8,
        index: u64::from(h[10]) << 32 | u64::from(u32_at(16)),
        samples: u32_at(20),
        flags: u32_at(24),
        total: (total_low != u32::MAX).then(|| u64::from(h[11]) << 32 | u64::from(total_low)),
    })
}

/// A WavPack stream decoded to interleaved stereo `f32`.
pub struct WavPackDecoder {
    source: Box<dyn AudioSourceStream>,
    sample_rate: u32,
    channels: usize,
    total: Option<u64>,
    /// Decoded samples of the current block, interleaved, and the read position in them.
    block: Vec<f32>,
    at: usize,
    /// Frames of the current block to drop (after a seek inside it).
    skip: usize,
    position: u64,
}

impl WavPackDecoder {
    pub fn open(mut source: Box<dyn AudioSourceStream>) -> Result<Self, DecodeError> {
        let mut dec = Self {
            source: Box::new(std::io::Cursor::new(Vec::new())),
            sample_rate: 0,
            channels: 0,
            total: None,
            block: Vec::new(),
            at: 0,
            skip: 0,
            position: 0,
        };
        source
            .seek(SeekFrom::Start(0))
            .map_err(|e| err(e.to_string()))?;
        dec.source = source;
        // The first audio block gives rate, channels and length.
        while dec.block.is_empty() {
            if !dec.next_block()? {
                return Err(err("no audio blocks"));
            }
        }
        Ok(dec)
    }

    fn read_header(&mut self) -> Result<Option<Header>, DecodeError> {
        let mut h = [0u8; HEADER];
        match self.source.read_exact(&mut h) {
            Ok(()) => Ok(parse_header(&h)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(err(e.to_string())),
        }
    }

    /// Decodes the next block into `self.block`; `false` at the end of the audio.
    fn next_block(&mut self) -> Result<bool, DecodeError> {
        let Some(h) = self.read_header()? else {
            return Ok(false); // end of file, or the APEv2 tag that follows the audio
        };
        if h.len < HEADER || h.len > MAX_BLOCK {
            return Err(err("corrupt block size"));
        }
        if h.flags & (FLAG_INITIAL | FLAG_FINAL) != FLAG_INITIAL | FLAG_FINAL {
            return Err(err("more than two channels are not supported"));
        }
        let mut bytes = vec![0u8; h.len];
        self.source
            .read_exact(&mut bytes[HEADER..])
            .map_err(|e| err(e.to_string()))?;
        if h.samples == 0 {
            self.block.clear();
            self.at = 0;
            return Ok(true); // metadata-only block
        }
        // Rebuild the header in front so the block is a stream of its own for `wavicle`.
        self.source
            .seek(SeekFrom::Current(-(h.len as i64)))
            .map_err(|e| err(e.to_string()))?;
        self.source
            .read_exact(&mut bytes)
            .map_err(|e| err(e.to_string()))?;
        let d = wavicle::decode::decode_stream(&bytes).map_err(|e| err(format!("{e:?}")))?;
        if self.sample_rate == 0 {
            self.sample_rate = d.sample_rate;
            self.channels = d.channels as usize;
            self.total = h.total;
        }
        let scale = 1.0 / (1u64 << (d.bits_per_sample.clamp(1, 32) - 1)) as f32;
        let to_f32 = |s: i32| {
            if d.is_float {
                f32::from_bits(s as u32)
            } else {
                s as f32 * scale
            }
        };
        self.block.clear();
        match self.channels {
            1 => self
                .block
                .extend(d.samples.iter().flat_map(|&s| [to_f32(s), to_f32(s)])),
            _ => self.block.extend(d.samples.iter().map(|&s| to_f32(s))),
        }
        self.at = self.skip.min(self.block.len() / 2) * 2;
        self.skip = 0;
        Ok(true)
    }
}

impl PcmStreamPort for WavPackDecoder {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, DecodeError> {
        let mut written = 0;
        while written + 1 < out.len() {
            if self.at >= self.block.len() && !self.next_block()? {
                break;
            }
            let n = (self.block.len() - self.at).min((out.len() - written) & !1);
            out[written..written + n].copy_from_slice(&self.block[self.at..self.at + n]);
            self.at += n;
            written += n;
        }
        self.position += (written / 2) as u64;
        Ok(written)
    }

    fn duration(&self) -> Option<Duration> {
        let total = self.total?;
        Some(Duration::from_secs_f64(
            total as f64 / f64::from(self.sample_rate),
        ))
    }

    fn elapsed(&self) -> Duration {
        Duration::from_secs_f64(self.position as f64 / f64::from(self.sample_rate))
    }

    fn seek(&mut self, target: Duration) -> Result<Duration, DecodeError> {
        let frame = (target.as_secs_f64() * f64::from(self.sample_rate)).round() as u64;
        let frame = self.total.map_or(frame, |t| frame.min(t));
        self.source
            .seek(SeekFrom::Start(0))
            .map_err(|e| err(e.to_string()))?;
        // Skip whole blocks by their headers until the one holding `frame`.
        loop {
            let Some(h) = self.read_header()? else {
                self.block.clear();
                self.at = 0;
                break;
            };
            if h.samples > 0 && frame < h.index + u64::from(h.samples) {
                self.source
                    .seek(SeekFrom::Current(-(HEADER as i64)))
                    .map_err(|e| err(e.to_string()))?;
                self.skip = (frame.saturating_sub(h.index)) as usize;
                self.next_block()?;
                break;
            }
            self.source
                .seek(SeekFrom::Current((h.len - HEADER) as i64))
                .map_err(|e| err(e.to_string()))?;
        }
        self.position = frame;
        Ok(self.elapsed())
    }
}
