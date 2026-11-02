//! File writers for mix recordings: streaming WAV (IEEE float) and FLAC (24-bit). Both write a
//! header first and fix its sizes when finished, so a long mix never sits in memory.

#![forbid(unsafe_code)]

use super::recorder::MixMetadata;
use flacenc::bitsink::ByteSink;
use flacenc::component::{BitRepr, Stream};
use flacenc::error::Verify;
use flacenc::source::{Context, Fill, FrameBuf};
use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

/// Frames per FLAC frame (the reference encoder's default).
const FLAC_BLOCK: usize = 4096;
const FLAC_BITS: usize = 24;
const FULL_24: f32 = 8_388_607.0;

/// Where recorded samples go.
pub trait Sink: Send {
    /// Appends interleaved stereo samples.
    fn write(&mut self, samples: &[f32]) -> io::Result<()>;
    /// Fixes the header and closes the file.
    fn finish(self: Box<Self>) -> io::Result<()>;
}

/// 32-bit float WAV.
pub struct WavSink {
    out: BufWriter<File>,
    rate: u32,
    samples: u64,
}

impl WavSink {
    /// Creates `path` and writes a header to fix later.
    ///
    /// # Errors
    /// The file cannot be created.
    pub fn create(path: &Path, rate: u32) -> io::Result<Self> {
        let mut sink = Self {
            out: BufWriter::with_capacity(1 << 16, File::create(path)?),
            rate,
            samples: 0,
        };
        sink.header()?;
        Ok(sink)
    }

    fn header(&mut self) -> io::Result<()> {
        // A WAV's sizes are 32-bit; a mix longer than about three hours keeps the maximum.
        let data = u32::try_from(self.samples * 4).unwrap_or(u32::MAX - 64);
        let frames = u32::try_from(self.samples / 2).unwrap_or(u32::MAX);
        let o = &mut self.out;
        o.write_all(b"RIFF")?;
        o.write_all(&(4 + 26 + 12 + 8 + data).to_le_bytes())?;
        o.write_all(b"WAVEfmt ")?;
        o.write_all(&18u32.to_le_bytes())?;
        o.write_all(&3u16.to_le_bytes())?; // IEEE float
        o.write_all(&2u16.to_le_bytes())?;
        o.write_all(&self.rate.to_le_bytes())?;
        o.write_all(&(self.rate * 8).to_le_bytes())?;
        o.write_all(&8u16.to_le_bytes())?;
        o.write_all(&32u16.to_le_bytes())?;
        o.write_all(&0u16.to_le_bytes())?;
        o.write_all(b"fact")?;
        o.write_all(&4u32.to_le_bytes())?;
        o.write_all(&frames.to_le_bytes())?;
        o.write_all(b"data")?;
        o.write_all(&data.to_le_bytes())
    }
}

impl Sink for WavSink {
    fn write(&mut self, samples: &[f32]) -> io::Result<()> {
        for s in samples {
            self.out.write_all(&s.to_le_bytes())?;
        }
        self.samples += samples.len() as u64;
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> io::Result<()> {
        self.out.seek(SeekFrom::Start(0))?;
        self.header()?;
        self.out.flush()?;
        self.out.get_ref().sync_all()
    }
}

/// 24-bit FLAC, encoded one frame at a time.
pub struct FlacSink {
    out: BufWriter<File>,
    header: Stream,
    config: flacenc::error::Verified<flacenc::config::Encoder>,
    pending: Vec<i32>,
    frame: FrameBuf,
    context: Context,
}

fn flac_err<E: std::fmt::Debug>(e: E) -> io::Error {
    io::Error::other(format!("FLAC: {e:?}"))
}

impl FlacSink {
    /// Creates `path` and writes a header to fix later.
    ///
    /// # Errors
    /// The file cannot be created.
    pub fn create(path: &Path, rate: u32) -> io::Result<Self> {
        let mut header = Stream::new(rate as usize, 2, FLAC_BITS).map_err(flac_err)?;
        header
            .stream_info_mut()
            .set_block_sizes(FLAC_BLOCK, FLAC_BLOCK)
            .map_err(flac_err)?;
        let mut config = flacenc::config::Encoder::default();
        config.block_size = FLAC_BLOCK;
        let config = config.into_verified().map_err(|(_, e)| flac_err(e))?;
        let mut sink = Self {
            out: BufWriter::with_capacity(1 << 16, File::create(path)?),
            header,
            config,
            pending: Vec::with_capacity(FLAC_BLOCK * 2),
            frame: FrameBuf::with_size(2, FLAC_BLOCK).map_err(flac_err)?,
            context: Context::new(FLAC_BITS, 2),
        };
        sink.write_header()?;
        Ok(sink)
    }

    fn write_header(&mut self) -> io::Result<()> {
        let mut bytes = ByteSink::new();
        self.header.write(&mut bytes).map_err(flac_err)?;
        self.out.write_all(bytes.as_slice())
    }

    fn encode(&mut self, frames: usize) -> io::Result<()> {
        let samples = &self.pending[..frames * 2];
        self.frame.fill_interleaved(samples).map_err(flac_err)?;
        self.context.fill_interleaved(samples).map_err(flac_err)?;
        let number = self.context.current_frame_number().unwrap_or(0);
        let frame = flacenc::encode_fixed_size_frame(
            &self.config,
            &self.frame,
            number,
            self.header.stream_info(),
        )
        .map_err(flac_err)?;
        self.header.stream_info_mut().update_frame_info(&frame);
        let mut bytes = ByteSink::new();
        frame.write(&mut bytes).map_err(flac_err)?;
        self.out.write_all(bytes.as_slice())?;
        self.pending.drain(..frames * 2);
        Ok(())
    }
}

impl Sink for FlacSink {
    fn write(&mut self, samples: &[f32]) -> io::Result<()> {
        for s in samples {
            self.pending
                .push((s.clamp(-1.0, 1.0) * FULL_24).round() as i32);
            if self.pending.len() == FLAC_BLOCK * 2 {
                self.encode(FLAC_BLOCK)?;
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> io::Result<()> {
        let left = self.pending.len() / 2;
        if left > 0 {
            self.encode(left)?;
        }
        let digest = self.context.md5_digest();
        let total = self.context.total_samples();
        let info = self.header.stream_info_mut();
        info.set_md5_digest(&digest);
        info.set_total_samples(total);
        self.out.seek(SeekFrom::Start(0))?;
        self.write_header()?;
        self.out.flush()?;
        self.out.get_ref().sync_all()
    }
}

/// Tags a finished recording as a mix.
///
/// # Errors
/// The file cannot be read or written by the tagger.
pub fn tag_mix(path: &Path, meta: &MixMetadata) -> Result<(), String> {
    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;
    use lofty::tag::{Accessor, Tag, TagExt};
    let mut file = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    if file.primary_tag().is_none() {
        let kind = file.primary_tag_type();
        file.insert_tag(Tag::new(kind));
    }
    let tag = file.primary_tag_mut().ok_or("no tag for this file type")?;
    tag.set_title(meta.title.clone());
    tag.set_artist(meta.artist.clone());
    tag.set_genre("DJ Mix".into());
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())
}
