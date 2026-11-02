//! DSD playback (TASK-231): DSF and DSDIFF (`.dff`) files with uncompressed 1-bit audio, converted
//! to PCM at 1/32 of the DSD rate (DSD64 → 88.2 kHz; the engine resamples from there).
//! Symphonia has no DSD support, so the decoder factory routes these files here.

#![forbid(unsafe_code)]

mod filter;

use crate::stream::downmix_frame;
use amp_ports::{AudioSourceStream, DecodeError, PcmStreamPort};
use filter::{Channel, BYTES_PER_OUTPUT, WARMUP_OUTPUTS};
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

/// How the 1-bit samples are laid out in the data chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Layout {
    /// DSF: per-channel blocks of `block` bytes; `lsb_first` bit order per byte.
    Blocks { block: usize, lsb_first: bool },
    /// DSDIFF: channels interleaved byte by byte, MSB first.
    Interleaved,
}

/// What the file header says.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Info {
    channels: usize,
    dsd_rate: u32,
    /// 1-bit samples per channel.
    samples: u64,
    data_offset: u64,
    layout: Layout,
}

/// Whether `head` starts a DSF (`DSD `) or DSDIFF (`FRM8`) file.
pub fn is_dsd(head: &[u8]) -> bool {
    head.starts_with(b"DSD ") || head.starts_with(b"FRM8")
}

fn err(msg: impl Into<String>) -> DecodeError {
    DecodeError::UnsupportedFormat(format!("DSD: {}", msg.into()))
}

/// Largest header chunk or audio block read in one go. Sizes come from the file, so a damaged
/// one could ask for gigabytes (TASK-185); real DSD headers and blocks are a few KiB.
const MAX_READ: usize = 1 << 20;
/// DSD files carry at most 6 channels in practice; 8 leaves room.
const MAX_CHANNELS: usize = 8;
/// DSF block size per channel (4096 in every real file).
const MAX_BLOCK: usize = 1 << 16;
/// DSD64 is 2.8224 MHz; DSD1024 about 45 MHz.
const DSD_RATES: std::ops::RangeInclusive<u32> = 2_000_000..=50_000_000;

fn read_exact<R: Read>(r: &mut R, n: usize) -> Result<Vec<u8>, DecodeError> {
    if n > MAX_READ {
        return Err(err(format!(
            "a {n}-byte chunk is too large; the file is damaged"
        )));
    }
    let mut buf = vec![0; n];
    r.read_exact(&mut buf).map_err(|e| err(e.to_string()))?;
    Ok(buf)
}

fn parse_dsf<R: Read + Seek>(r: &mut R) -> Result<Info, DecodeError> {
    let head = read_exact(r, 28)?;
    let le64 = |b: &[u8]| u64::from_le_bytes(b[..8].try_into().unwrap_or_default());
    let le32 = |b: &[u8]| u32::from_le_bytes(b[..4].try_into().unwrap_or_default());
    let mut pos = le64(&head[4..]);
    r.seek(SeekFrom::Start(pos))
        .map_err(|e| err(e.to_string()))?;
    let fmt = read_exact(r, 52)?;
    if &fmt[..4] != b"fmt " || le32(&fmt[16..]) != 0 {
        return Err(err("unsupported DSF format"));
    }
    // fmt chunk: id, size, version, format id, channel type, channels, rate, bits, samples, block.
    let bits = le32(&fmt[32..]);
    let info = Info {
        channels: le32(&fmt[24..]) as usize,
        dsd_rate: le32(&fmt[28..]),
        samples: le64(&fmt[36..]),
        data_offset: 0,
        layout: Layout::Blocks {
            block: le32(&fmt[44..]) as usize,
            lsb_first: bits == 1,
        },
    };
    if info.channels == 0 || info.channels > MAX_CHANNELS || !DSD_RATES.contains(&info.dsd_rate) {
        return Err(err("damaged DSF header (channels or rate)"));
    }
    if let Layout::Blocks { block, .. } = info.layout {
        if block == 0 || block > MAX_BLOCK {
            return Err(err("damaged DSF header (block size)"));
        }
    }
    pos = pos.saturating_add(le64(&fmt[4..]));
    r.seek(SeekFrom::Start(pos))
        .map_err(|e| err(e.to_string()))?;
    if read_exact(r, 12)?[..4] != *b"data" {
        return Err(err("missing DSF data chunk"));
    }
    Ok(Info {
        data_offset: pos + 12,
        ..info
    })
}

fn parse_dff<R: Read + Seek>(r: &mut R) -> Result<Info, DecodeError> {
    let be64 = |b: &[u8]| u64::from_be_bytes(b[..8].try_into().unwrap_or_default());
    let head = read_exact(r, 16)?;
    if &head[12..16] != b"DSD " {
        return Err(err("not a DSD form"));
    }
    let end = 12 + be64(&head[4..]);
    let (mut channels, mut rate, mut data) = (0usize, 0u32, None);
    let mut pos = 16;
    while pos + 12 <= end {
        r.seek(SeekFrom::Start(pos))
            .map_err(|e| err(e.to_string()))?;
        let ch = read_exact(r, 12)?;
        let size = be64(&ch[4..]);
        match &ch[..4] {
            b"PROP" => {
                let prop = read_exact(r, usize::try_from(size).unwrap_or(usize::MAX))?;
                let mut p = 4; // "SND "
                while p + 12 <= prop.len() {
                    let sub = be64(&prop[p + 4..]) as usize;
                    let body = &prop[p + 12..(p + 12).saturating_add(sub).min(prop.len())];
                    match &prop[p..p + 4] {
                        b"FS  " if body.len() >= 4 => {
                            rate = u32::from_be_bytes(body[..4].try_into().unwrap_or_default());
                        }
                        b"CHNL" if body.len() >= 2 => {
                            channels = usize::from(u16::from_be_bytes([body[0], body[1]]));
                        }
                        b"CMPR" if !body.starts_with(b"DSD ") => {
                            return Err(err("compressed DSDIFF (DST) is not supported"));
                        }
                        _ => {}
                    }
                    p = p
                        .saturating_add(12)
                        .saturating_add(sub)
                        .saturating_add(sub % 2);
                }
            }
            b"DSD " => data = Some((pos + 12, size)),
            _ => {}
        }
        pos = pos
            .saturating_add(12)
            .saturating_add(size)
            .saturating_add(size % 2);
    }
    let (offset, len) = data.ok_or_else(|| err("missing DSD data chunk"))?;
    if channels == 0 || channels > MAX_CHANNELS || !DSD_RATES.contains(&rate) {
        return Err(err("missing or damaged sample rate or channel count"));
    }
    Ok(Info {
        channels,
        dsd_rate: rate,
        samples: len * 8 / channels as u64,
        data_offset: offset,
        layout: Layout::Interleaved,
    })
}

/// Bit-reverses a byte (DSF stores the oldest bit in the LSB).
fn reverse(b: u8) -> u8 {
    b.reverse_bits()
}

/// A DSD file decoded to interleaved stereo PCM.
pub struct DsdDecoder {
    source: Box<dyn AudioSourceStream>,
    info: Info,
    filters: Vec<Channel>,
    /// Per-channel bytes read but not yet filtered (MSB first).
    pending: Vec<Vec<u8>>,
    /// Read position in every channel's `pending` (all channels move together).
    cursor: usize,
    /// Next byte (per channel) to read from the file.
    next_byte: u64,
    /// Output frames still to drop after a seek (filter warm-up).
    discard: u64,
    frames_out: u64,
    frame: Vec<f32>,
}

impl DsdDecoder {
    /// Opens a DSF or DSDIFF stream (the header decides which).
    pub fn open(mut source: Box<dyn AudioSourceStream>) -> Result<Self, DecodeError> {
        let mut magic = [0u8; 4];
        source
            .read_exact(&mut magic)
            .map_err(|e| err(e.to_string()))?;
        source
            .seek(SeekFrom::Start(0))
            .map_err(|e| err(e.to_string()))?;
        let info = match &magic {
            b"DSD " => parse_dsf(&mut source)?,
            b"FRM8" => parse_dff(&mut source)?,
            _ => return Err(err("not a DSF or DSDIFF file")),
        };
        if !(1..=6).contains(&info.channels) || info.dsd_rate < 2_822_400 {
            return Err(err("unsupported channel count or rate"));
        }
        let mut dec = Self {
            source,
            filters: vec![Channel::default(); info.channels],
            pending: vec![Vec::new(); info.channels],
            cursor: 0,
            info,
            next_byte: 0,
            discard: 0,
            frames_out: 0,
            frame: vec![0.0; info.channels],
        };
        dec.position_at(0)?;
        Ok(dec)
    }

    fn total_frames(&self) -> u64 {
        self.info.samples / 8 / BYTES_PER_OUTPUT as u64
    }

    /// Positions reading at per-channel byte `byte` with fresh filters.
    fn position_at(&mut self, byte: u64) -> Result<(), DecodeError> {
        self.filters
            .iter_mut()
            .for_each(|f| *f = Channel::default());
        self.pending.iter_mut().for_each(Vec::clear);
        self.cursor = 0;
        let skip_in_block = match self.info.layout {
            Layout::Blocks { block, .. } => {
                let b = block as u64;
                let start = self.info.data_offset + byte / b * b * self.info.channels as u64;
                self.source
                    .seek(SeekFrom::Start(start))
                    .map_err(|e| err(e.to_string()))?;
                self.next_byte = byte / b * b;
                (byte % b) as usize
            }
            Layout::Interleaved => {
                let start = self.info.data_offset + byte * self.info.channels as u64;
                self.source
                    .seek(SeekFrom::Start(start))
                    .map_err(|e| err(e.to_string()))?;
                self.next_byte = byte;
                0
            }
        };
        if skip_in_block > 0 {
            self.fill()?;
            self.cursor = skip_in_block.min(self.pending[0].len());
        }
        Ok(())
    }

    /// Reads the next block (DSF) or a chunk of interleaved bytes (DSDIFF) into `pending`.
    fn fill(&mut self) -> Result<bool, DecodeError> {
        let channels = self.info.channels;
        let left = (self.info.samples / 8).saturating_sub(self.next_byte);
        if left == 0 {
            return Ok(false);
        }
        // Drop what was already filtered before appending (once per block, not per sample).
        let used = self.cursor;
        self.pending.iter_mut().for_each(|p| {
            p.drain(..used);
        });
        self.cursor = 0;
        match self.info.layout {
            Layout::Blocks { block, lsb_first } => {
                let raw = read_exact(&mut self.source, block * channels)?;
                let take = (block as u64).min(left) as usize;
                for (ch, pending) in self.pending.iter_mut().enumerate() {
                    let bytes = &raw[ch * block..ch * block + take];
                    pending.extend(
                        bytes
                            .iter()
                            .map(|&b| if lsb_first { reverse(b) } else { b }),
                    );
                }
                self.next_byte += take as u64;
            }
            Layout::Interleaved => {
                let take = (4096u64).min(left) as usize;
                let raw = read_exact(&mut self.source, take * channels)?;
                for (i, &b) in raw.iter().enumerate() {
                    self.pending[i % channels].push(b);
                }
                self.next_byte += take as u64;
            }
        }
        Ok(true)
    }
}

impl PcmStreamPort for DsdDecoder {
    fn sample_rate(&self) -> u32 {
        self.info.dsd_rate / 32
    }

    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, DecodeError> {
        let mut written = 0;
        while written + 1 < out.len() && self.frames_out < self.total_frames() {
            if self.pending[0].len() - self.cursor < BYTES_PER_OUTPUT && !self.fill()? {
                break;
            }
            if self.pending[0].len() - self.cursor < BYTES_PER_OUTPUT {
                continue;
            }
            let at = self.cursor;
            for (ch, filter) in self.filters.iter_mut().enumerate() {
                let p = &self.pending[ch];
                self.frame[ch] = filter.push([p[at], p[at + 1], p[at + 2], p[at + 3]]);
            }
            self.cursor += BYTES_PER_OUTPUT;
            if self.discard > 0 {
                self.discard -= 1;
                continue;
            }
            let (l, r) = downmix_frame(&self.frame, 0, self.info.channels);
            out[written] = l;
            out[written + 1] = r;
            written += 2;
            self.frames_out += 1;
        }
        Ok(written)
    }

    fn duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.info.samples as f64 / f64::from(self.info.dsd_rate),
        ))
    }

    fn elapsed(&self) -> Duration {
        Duration::from_secs_f64(self.frames_out as f64 / f64::from(self.sample_rate()))
    }

    fn seek(&mut self, target: Duration) -> Result<Duration, DecodeError> {
        let frame = ((target.as_secs_f64() * f64::from(self.sample_rate())) as u64)
            .min(self.total_frames());
        // Start a filter length early so the output at the target is fully warmed up.
        let warmup = WARMUP_OUTPUTS as u64;
        let start = frame.saturating_sub(warmup);
        self.position_at(start * BYTES_PER_OUTPUT as u64)?;
        self.discard = frame - start;
        self.frames_out = frame;
        Ok(self.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filter::DSD_SILENCE;

    #[test]
    fn silence_pattern_filters_to_zero() {
        let mut ch = Channel::default();
        let s = ch.push([DSD_SILENCE; BYTES_PER_OUTPUT]);
        assert!(s.abs() < 0.01, "{s}");
    }

    #[test]
    fn all_ones_settle_at_full_scale() {
        let mut ch = Channel::default();
        let mut last = 0.0;
        for _ in 0..WARMUP_OUTPUTS {
            last = ch.push([0xFF; BYTES_PER_OUTPUT]);
        }
        assert!((last - 1.0).abs() < 1e-3, "{last}");
    }
}
