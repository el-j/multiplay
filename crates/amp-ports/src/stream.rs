//! Concurrent audio stream precaching buffer (`TASK-135`, `TASK-136`).
//!
//! Provides:
//! - `Condvar`-backed blocking reads with configurable timeout (`ErrorKind::TimedOut`) and
//!   cancellation (`ErrorKind::Interrupted`).
//! - Bounded-memory live sliding window mode (`new_live_ring`, default 8 MB) for infinite
//!   internet radio streams.
//! - Disk-spill mode (`with_disk_spill`) for finite streams exceeding the RAM budget.
//! - Automatic producer cancellation when [`PrecacheStreamReader`] is dropped on track change.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Default sliding-window size for live internet radio streams (8 MB, `TASK-136`).
pub const DEFAULT_LIVE_WINDOW_BYTES: usize = 8 * 1024 * 1024;

/// Default initial buffering watermark (64 KB) used to compute `buffering_percent()`.
pub const DEFAULT_BUFFER_WATERMARK_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct SharedPrecacheBuffer {
    inner: Arc<(Mutex<PrecacheBufferState>, Condvar)>,
    cancelled: Arc<AtomicBool>,
    read_timeout: Option<Duration>,
}

struct PrecacheBufferState {
    /// In-memory byte storage (`VecDeque` enables $O(1)$ front eviction in live sliding-window mode).
    ram: VecDeque<u8>,
    /// Absolute stream byte offset corresponding to `ram[0]`.
    base_offset: u64,
    /// Current reader byte offset in the logical stream.
    cursor: u64,
    /// Total bytes pushed to the stream (including evicted live prefix or disk-spilled suffix).
    total_pushed: u64,
    /// Maximum bytes kept in `ram` before either evicting old bytes (`live_ring`) or spilling to disk.
    ram_limit: usize,
    /// True when this buffer operates as a sliding-window live stream (unseekable past evicted window).
    live_ring: bool,
    /// Optional disk spill file for finite streams exceeding `ram_limit`.
    spill_file: Option<File>,
    spill_path: Option<PathBuf>,
    /// Watermark in bytes for reporting 0..=100% buffering progress.
    watermark_bytes: usize,
    /// True while a reader is waiting for bytes.
    currently_buffering: bool,
    /// True once the producer has finished pushing all bytes.
    stream_ended: bool,
}

impl Drop for PrecacheBufferState {
    fn drop(&mut self) {
        self.spill_file = None;
        if let Some(ref p) = self.spill_path {
            let _ = std::fs::remove_file(p);
        }
    }
}

impl SharedPrecacheBuffer {
    /// Creates a new precache buffer with preallocated capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new((
                Mutex::new(PrecacheBufferState {
                    ram: VecDeque::with_capacity(capacity.min(16 * 1024 * 1024)),
                    base_offset: 0,
                    cursor: 0,
                    total_pushed: 0,
                    ram_limit: usize::MAX,
                    live_ring: false,
                    spill_file: None,
                    spill_path: None,
                    watermark_bytes: DEFAULT_BUFFER_WATERMARK_BYTES.min(capacity.max(1)),
                    currently_buffering: false,
                    stream_ended: false,
                }),
                Condvar::new(),
            )),
            cancelled: Arc::new(AtomicBool::new(false)),
            read_timeout: None,
        }
    }

    /// Creates a blocking precache buffer whose `Read::read` waits on a `Condvar` up to `timeout`
    /// instead of returning `WouldBlock` (`TASK-135`).
    #[must_use]
    pub fn new_blocking(capacity: usize, timeout: Duration) -> Self {
        Self::new(capacity).with_read_timeout(timeout)
    }

    /// Creates a bounded-memory sliding-window buffer for live/infinite streams (`TASK-136`).
    #[must_use]
    pub fn new_live_ring(window_bytes: usize, timeout: Duration) -> Self {
        let cap = window_bytes.max(4096);
        let buf = Self::new(cap).with_read_timeout(timeout);
        {
            let (lock, _) = &*buf.inner;
            let mut st = lock.lock().unwrap();
            st.ram_limit = cap;
            st.live_ring = true;
        }
        buf
    }

    /// Configures a blocking timeout for `Read::read` (`TASK-135`).
    #[must_use]
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    /// Configures finite-stream disk spill once in-memory bytes exceed `ram_limit_bytes` (`TASK-136`).
    pub fn with_disk_spill(
        self,
        ram_limit_bytes: usize,
        spill_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let path = spill_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&path)?;
        {
            let (lock, _) = &*self.inner;
            let mut st = lock.lock().unwrap();
            st.ram_limit = ram_limit_bytes.max(1024);
            st.live_ring = false;
            st.spill_file = Some(file);
            st.spill_path = Some(path);
        }
        Ok(self)
    }

    /// Wraps this buffer in a [`PrecacheStreamReader`] that cancels the producer when dropped.
    #[must_use]
    pub fn into_reader(self) -> PrecacheStreamReader {
        PrecacheStreamReader { buffer: self }
    }

    /// Signals cancellation and wakes any blocked readers (`TASK-135`).
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let (_, cvar) = &*self.inner;
        cvar.notify_all();
    }

    /// Returns true if the stream has been cancelled (e.g., reader dropped on track skip).
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Appends incoming audio stream bytes and wakes waiting readers.
    /// Returns `false` if the stream was cancelled so producer tasks can stop immediately.
    pub fn push_chunk(&self, chunk: &[u8]) -> bool {
        if self.is_cancelled() {
            return false;
        }
        let (lock, cvar) = &*self.inner;
        let mut st = lock.lock().unwrap();
        if st.live_ring {
            st.ram.extend(chunk.iter().copied());
            st.total_pushed += chunk.len() as u64;
            if st.ram.len() > st.ram_limit {
                let overflow = st.ram.len() - st.ram_limit;
                st.ram.drain(..overflow);
                st.base_offset += overflow as u64;
                if st.cursor < st.base_offset {
                    st.cursor = st.base_offset;
                }
            }
        } else if st.ram.len() < st.ram_limit {
            let space_in_ram = (st.ram_limit - st.ram.len()).min(chunk.len());
            st.ram.extend(chunk[..space_in_ram].iter().copied());
            let remainder = &chunk[space_in_ram..];
            if !remainder.is_empty() {
                if let Some(ref mut f) = st.spill_file {
                    let _ = f.seek(SeekFrom::End(0));
                    let _ = f.write_all(remainder);
                } else {
                    st.ram.extend(remainder.iter().copied());
                }
            }
            st.total_pushed += chunk.len() as u64;
        } else if let Some(ref mut f) = st.spill_file {
            let _ = f.seek(SeekFrom::End(0));
            let _ = f.write_all(chunk);
            st.total_pushed += chunk.len() as u64;
        } else {
            st.ram.extend(chunk.iter().copied());
            st.total_pushed += chunk.len() as u64;
        }
        cvar.notify_all();
        !self.is_cancelled()
    }

    /// Marks the stream as complete and wakes all waiting readers.
    pub fn mark_finished(&self) {
        let (lock, cvar) = &*self.inner;
        let mut st = lock.lock().unwrap();
        st.stream_ended = true;
        st.currently_buffering = false;
        cvar.notify_all();
    }

    /// Total logical bytes pushed into the stream.
    #[must_use]
    pub fn total_bytes_cached(&self) -> usize {
        let (lock, _) = &*self.inner;
        lock.lock().unwrap().total_pushed as usize
    }

    /// Bytes currently resident in RAM (never exceeds `ram_limit` when bounded).
    #[must_use]
    pub fn ram_bytes_resident(&self) -> usize {
        let (lock, _) = &*self.inner;
        lock.lock().unwrap().ram.len()
    }

    /// Returns true if the stream has finished downloading.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        let (lock, _) = &*self.inner;
        lock.lock().unwrap().stream_ended
    }

    /// Returns current buffering progress in `0..=100` (`TASK-135`).
    #[must_use]
    pub fn buffering_percent(&self) -> u8 {
        let (lock, _) = &*self.inner;
        let st = lock.lock().unwrap();
        if st.stream_ended {
            return 100;
        }
        let available = st.total_pushed.saturating_sub(st.cursor) as usize;
        let wm = st.watermark_bytes.max(1);
        ((available.saturating_mul(100)) / wm).min(100) as u8
    }

    /// Clears the buffer and resets playback position.
    pub fn clear(&self) {
        let (lock, cvar) = &*self.inner;
        let mut st = lock.lock().unwrap();
        st.ram.clear();
        st.base_offset = 0;
        st.cursor = 0;
        st.total_pushed = 0;
        st.stream_ended = false;
        st.currently_buffering = false;
        if let Some(ref mut f) = st.spill_file {
            let _ = f.set_len(0);
        }
        cvar.notify_all();
    }

    /// Returns the current read cursor offset.
    #[must_use]
    pub fn current_cursor(&self) -> usize {
        let (lock, _) = &*self.inner;
        lock.lock().unwrap().cursor as usize
    }
}

impl Read for SharedPrecacheBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let (lock, cvar) = &*self.inner;
        let mut st = lock.lock().unwrap();
        let deadline = self.read_timeout.map(|d| Instant::now() + d);

        loop {
            if self.is_cancelled() {
                st.currently_buffering = false;
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Precache stream cancelled",
                ));
            }

            if st.cursor < st.base_offset {
                st.cursor = st.base_offset;
            }
            let available = st.total_pushed.saturating_sub(st.cursor) as usize;
            if available > 0 {
                st.currently_buffering = false;
                let to_read = buf.len().min(available);
                let rel = (st.cursor - st.base_offset) as usize;

                if rel < st.ram.len() {
                    let from_ram = to_read.min(st.ram.len() - rel);
                    let (s1, s2) = st.ram.as_slices();
                    if rel < s1.len() {
                        let n1 = from_ram.min(s1.len() - rel);
                        buf[..n1].copy_from_slice(&s1[rel..rel + n1]);
                        if from_ram > n1 {
                            let n2 = from_ram - n1;
                            buf[n1..n1 + n2].copy_from_slice(&s2[..n2]);
                        }
                    } else {
                        let s2_rel = rel - s1.len();
                        buf[..from_ram].copy_from_slice(&s2[s2_rel..s2_rel + from_ram]);
                    }
                    st.cursor += from_ram as u64;
                    return Ok(from_ram);
                } else {
                    let disk_offset = st.cursor - (st.base_offset + st.ram.len() as u64);
                    if let Some(ref mut f) = st.spill_file {
                        f.seek(SeekFrom::Start(disk_offset))?;
                        let n = f.read(&mut buf[..to_read])?;
                        st.cursor += n as u64;
                        return Ok(n);
                    }
                }
            }

            if st.stream_ended {
                st.currently_buffering = false;
                return Ok(0);
            }

            let Some(dl) = deadline else {
                st.currently_buffering = true;
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Precache buffer underflow: awaiting incoming network chunks",
                ));
            };

            let now = Instant::now();
            if now >= dl {
                st.currently_buffering = true;
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Precache stream timed out waiting for network data",
                ));
            }
            st.currently_buffering = true;
            let (new_st, _) = cvar.wait_timeout(st, dl - now).unwrap();
            st = new_st;
        }
    }
}

impl Seek for SharedPrecacheBuffer {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let (lock, _) = &*self.inner;
        let mut st = lock.lock().unwrap();
        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => st.cursor as i64 + offset,
            SeekFrom::End(offset) => st.total_pushed as i64 + offset,
        };

        if target < (st.base_offset as i64) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cannot seek before sliding-window base offset",
            ));
        }

        st.cursor = (target as u64).min(st.total_pushed);
        Ok(st.cursor)
    }
}

/// Reader handle that automatically cancels the underlying [`SharedPrecacheBuffer`] on `Drop` (`TASK-135`).
pub struct PrecacheStreamReader {
    buffer: SharedPrecacheBuffer,
}

impl PrecacheStreamReader {
    #[must_use]
    pub fn buffer(&self) -> &SharedPrecacheBuffer {
        &self.buffer
    }
}

impl Read for PrecacheStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.buffer.read(buf)
    }
}

impl Seek for PrecacheStreamReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.buffer.seek(pos)
    }
}

impl Drop for PrecacheStreamReader {
    fn drop(&mut self) {
        self.buffer.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precache_push_and_read() {
        let mut buffer = SharedPrecacheBuffer::new(1024);
        buffer.push_chunk(b"AUDIO_HEADER_DATA_1234");
        assert_eq!(buffer.total_bytes_cached(), 22);

        let mut out = [0u8; 12];
        let n = buffer.read(&mut out).expect("read failed");
        assert_eq!(n, 12);
        assert_eq!(&out, b"AUDIO_HEADER");

        let mut out2 = [0u8; 10];
        let n2 = buffer.read(&mut out2).expect("read failed");
        assert_eq!(n2, 10);
        assert_eq!(&out2, b"_DATA_1234");
    }

    #[test]
    fn test_precache_underflow_and_eof() {
        let mut buffer = SharedPrecacheBuffer::new(512);
        buffer.push_chunk(b"PARTIAL");

        let mut out = [0u8; 7];
        let n = buffer.read(&mut out).expect("read failed");
        assert_eq!(n, 7);

        let mut empty_read = [0u8; 4];
        let err = buffer.read(&mut empty_read).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);

        buffer.mark_finished();
        let eof = buffer.read(&mut empty_read).expect("eof read failed");
        assert_eq!(eof, 0);
    }

    #[test]
    fn precache_blocking_reader_waits_on_condvar_times_out_and_cancels_on_drop() {
        let buffer = SharedPrecacheBuffer::new_blocking(1024, Duration::from_millis(200));
        let writer = buffer.clone();

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            writer.push_chunk(b"CONDVAR_OK");
        });

        let mut reader = buffer.clone().into_reader();
        let mut out = [0u8; 10];
        let n = reader
            .read(&mut out)
            .expect("blocking read should wake on push");
        assert_eq!(n, 10);
        assert_eq!(&out, b"CONDVAR_OK");

        // Timeout returns ErrorKind::TimedOut (never WouldBlock)
        let mut timed_out_buf = SharedPrecacheBuffer::new_blocking(64, Duration::from_millis(40));
        let err = timed_out_buf.read(&mut out).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);

        // Dropping PrecacheStreamReader cancels the producer
        assert!(!buffer.is_cancelled());
        drop(reader);
        assert!(buffer.is_cancelled());
        assert!(!buffer.push_chunk(b"AFTER_DROP"));
    }

    #[test]
    fn test_precache_seek() {
        let mut buffer = SharedPrecacheBuffer::new(512);
        buffer.push_chunk(b"0123456789ABCDEF");

        buffer.seek(SeekFrom::Start(4)).unwrap();
        let mut out = [0u8; 4];
        buffer.read_exact(&mut out).unwrap();
        assert_eq!(&out, b"4567");

        buffer.seek(SeekFrom::Current(2)).unwrap();
        let mut out2 = [0u8; 2];
        buffer.read_exact(&mut out2).unwrap();
        assert_eq!(&out2, b"AB");

        buffer.seek(SeekFrom::End(-4)).unwrap();
        let mut out3 = [0u8; 4];
        buffer.read_exact(&mut out3).unwrap();
        assert_eq!(&out3, b"CDEF");
    }
}
