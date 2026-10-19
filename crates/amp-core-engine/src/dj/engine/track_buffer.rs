//! A deck's track, decoded into memory while it plays.
//!
//! A loader thread appends decoded stereo audio in fixed-size chunks; each finished chunk is
//! published once through a `OnceLock`, so the render thread reads any loaded frame without
//! locks or allocation, and seeking, loops and cues within the loaded part are instant.

#![forbid(unsafe_code)]

use amp_ports::PcmStreamPort;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

/// Frames per chunk (about 0.37 s at 44.1 kHz).
pub const CHUNK_FRAMES: usize = 16_384;
/// Chunk slots per track: about 3 hours at 48 kHz.
const MAX_CHUNKS: usize = 32_768;

/// Decoded track audio shared between its loader and a deck.
#[derive(Debug)]
pub struct TrackBuffer {
    sample_rate: AtomicU32,
    chunks: Box<[OnceLock<Box<[f32]>>]>,
    frames: AtomicUsize,
    complete: AtomicBool,
    failed: OnceLock<String>,
    cancelled: AtomicBool,
}

impl TrackBuffer {
    /// An empty buffer whose sample rate is set by the loader once the stream is open.
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            sample_rate: AtomicU32::new(0),
            chunks: (0..MAX_CHUNKS).map(|_| OnceLock::new()).collect(),
            frames: AtomicUsize::new(0),
            complete: AtomicBool::new(false),
            failed: OnceLock::new(),
            cancelled: AtomicBool::new(false),
        })
    }

    /// A complete buffer holding `samples` (interleaved stereo) at `sample_rate`.
    pub fn from_samples(sample_rate: u32, samples: &[f32]) -> Arc<Self> {
        let buf = Self::empty();
        let mut w = TrackWriter::new(Arc::clone(&buf), sample_rate);
        w.push(samples);
        w.finish();
        buf
    }

    /// Sample rate of the track, 0 until the loader has opened the stream.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate.load(Ordering::Acquire)
    }

    /// Frames that can be read now.
    pub fn loaded_frames(&self) -> usize {
        self.frames.load(Ordering::Acquire)
    }

    /// True once the whole track is decoded (or loading stopped early).
    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Acquire)
    }

    /// Why loading failed, if it did.
    pub fn error(&self) -> Option<&str> {
        self.failed.get().map(String::as_str)
    }

    /// True once [`Self::cancel`] was called.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Asks the loader to stop (the deck no longer needs this track).
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Stereo frame `i`, or `None` when it is not loaded (yet).
    #[inline]
    pub fn frame(&self, i: usize) -> Option<(f32, f32)> {
        if i >= self.loaded_frames() {
            return None;
        }
        let chunk = self.chunks.get(i / CHUNK_FRAMES)?.get()?;
        let at = (i % CHUNK_FRAMES) * 2;
        Some((chunk[at], chunk[at + 1]))
    }
}

/// The loader's side of a [`TrackBuffer`].
#[derive(Debug)]
pub struct TrackWriter {
    buf: Arc<TrackBuffer>,
    pending: Vec<f32>,
    next_chunk: usize,
}

impl TrackWriter {
    /// Starts writing into `buf`, publishing `sample_rate`.
    pub fn new(buf: Arc<TrackBuffer>, sample_rate: u32) -> Self {
        buf.sample_rate.store(sample_rate, Ordering::Release);
        Self {
            buf,
            pending: Vec::with_capacity(CHUNK_FRAMES * 2),
            next_chunk: 0,
        }
    }

    /// Appends interleaved stereo samples. Returns false when the buffer is full or cancelled.
    pub fn push(&mut self, mut samples: &[f32]) -> bool {
        while !samples.is_empty() {
            if self.buf.cancelled.load(Ordering::Relaxed) || self.next_chunk >= MAX_CHUNKS {
                return false;
            }
            let room = CHUNK_FRAMES * 2 - self.pending.len();
            let take = room.min(samples.len());
            self.pending.extend_from_slice(&samples[..take]);
            samples = &samples[take..];
            if self.pending.len() == CHUNK_FRAMES * 2 {
                self.publish();
            }
        }
        true
    }

    /// Publishes the last partial chunk and marks the track complete.
    pub fn finish(mut self) {
        if !self.pending.is_empty() && self.next_chunk < MAX_CHUNKS {
            self.pending.truncate(self.pending.len() / 2 * 2);
            self.publish();
        }
        self.buf.complete.store(true, Ordering::Release);
    }

    /// Marks loading as failed; what was decoded so far stays playable.
    pub fn fail(self, why: String) {
        let _ = self.buf.failed.set(why);
        self.finish();
    }

    fn publish(&mut self) {
        let frames = self.pending.len() / 2;
        let data = std::mem::replace(&mut self.pending, Vec::with_capacity(CHUNK_FRAMES * 2));
        let _ = self.buf.chunks[self.next_chunk].set(data.into_boxed_slice());
        self.next_chunk += 1;
        self.buf.frames.fetch_add(frames, Ordering::Release);
    }
}

/// Opens a stream with `open` on a background thread and decodes it into a new buffer, which is
/// returned at once so a deck can start playing while the rest of the track loads.
pub fn spawn_loader<F>(open: F) -> Arc<TrackBuffer>
where
    F: FnOnce() -> Result<Box<dyn PcmStreamPort>, String> + Send + 'static,
{
    let buf = TrackBuffer::empty();
    let target = Arc::clone(&buf);
    let _ = std::thread::Builder::new()
        .name("multiplay-dj-loader".into())
        .spawn(move || load(open, target));
    buf
}

fn load<F>(open: F, buf: Arc<TrackBuffer>)
where
    F: FnOnce() -> Result<Box<dyn PcmStreamPort>, String>,
{
    let mut stream = match open() {
        Ok(s) => s,
        Err(e) => return TrackWriter::new(buf, 0).fail(e),
    };
    let mut w = TrackWriter::new(buf, stream.sample_rate());
    let mut block = vec![0.0f32; 8_192];
    loop {
        match stream.read_interleaved(&mut block) {
            Ok(0) => return w.finish(),
            Ok(n) => {
                if !w.push(&block[..n]) {
                    return w.finish();
                }
            }
            Err(e) => return w.fail(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_buffer_frames_appear_chunk_by_chunk() {
        let buf = TrackBuffer::empty();
        let mut w = TrackWriter::new(Arc::clone(&buf), 44_100);
        let samples: Vec<f32> = (0..CHUNK_FRAMES * 2 + 20).map(|i| i as f32).collect();
        w.push(&samples);
        assert_eq!(buf.loaded_frames(), CHUNK_FRAMES);
        assert_eq!(buf.frame(3), Some((6.0, 7.0)));
        assert_eq!(
            buf.frame(CHUNK_FRAMES),
            None,
            "partial chunk not yet visible"
        );
        w.finish();
        assert_eq!(buf.loaded_frames(), CHUNK_FRAMES + 10);
        assert_eq!(
            buf.frame(CHUNK_FRAMES + 9),
            Some((
                (CHUNK_FRAMES * 2 + 18) as f32,
                (CHUNK_FRAMES * 2 + 19) as f32
            ))
        );
        assert!(buf.is_complete());
    }

    #[test]
    fn dj_cancelled_loader_stops_writing() {
        let buf = TrackBuffer::empty();
        let mut w = TrackWriter::new(Arc::clone(&buf), 48_000);
        buf.cancel();
        assert!(!w.push(&[0.0; 64]));
    }

    #[test]
    fn dj_failed_open_is_reported() {
        let buf = spawn_loader(|| Err("no such file".to_string()));
        for _ in 0..200 {
            if buf.is_complete() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(buf.error(), Some("no such file"));
        assert_eq!(buf.loaded_frames(), 0);
    }
}
