//! In-memory stream buffer for concurrent audio precaching.
//! Allows asynchronous network downloaders to push chunks while decoders read and seek.

#![forbid(unsafe_code)]

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
    /// Creates a new precache buffer with preallocated capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(PrecacheBufferState {
                data: Vec::with_capacity(capacity),
                cursor: 0,
                stream_ended: false,
            })),
        }
    }

    /// Appends incoming audio stream bytes to the internal buffer.
    pub fn push_chunk(&self, chunk: &[u8]) {
        let mut state = self.inner.write().expect("RwLock poisoned");
        state.data.extend_from_slice(chunk);
    }

    /// Marks the stream as complete (no more chunks will arrive).
    pub fn mark_finished(&self) {
        let mut state = self.inner.write().expect("RwLock poisoned");
        state.stream_ended = true;
    }

    /// Returns the total number of bytes currently cached in memory.
    pub fn total_bytes_cached(&self) -> usize {
        self.inner.read().expect("RwLock poisoned").data.len()
    }

    /// Returns true if the stream has finished downloading.
    pub fn is_finished(&self) -> bool {
        self.inner.read().expect("RwLock poisoned").stream_ended
    }

    /// Clears the buffer and resets playback position.
    pub fn clear(&self) {
        let mut state = self.inner.write().expect("RwLock poisoned");
        state.data.clear();
        state.cursor = 0;
        state.stream_ended = false;
    }

    /// Returns the current read cursor offset.
    pub fn current_cursor(&self) -> usize {
        self.inner.read().expect("RwLock poisoned").cursor
    }
}

impl Read for SharedPrecacheBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut state = self.inner.write().expect("RwLock poisoned");
        let available = state.data.len().saturating_sub(state.cursor);

        if available == 0 {
            if state.stream_ended {
                return Ok(0); // Clean EOF reached
            }
            // Signal decoder to yield momentarily while network reader pushes more chunks
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Precache buffer underflow: awaiting incoming network chunks",
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
        let mut state = self.inner.write().expect("RwLock poisoned");
        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => state.cursor as i64 + offset,
            SeekFrom::End(offset) => state.data.len() as i64 + offset,
        };

        if target < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid seek to negative position",
            ));
        }

        state.cursor = (target as usize).min(state.data.len());
        Ok(state.cursor as u64)
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

        // Next read with no data and stream not ended should return WouldBlock
        let mut empty_read = [0u8; 4];
        let err = buffer.read(&mut empty_read).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);

        // Mark finished -> should return Ok(0) (EOF)
        buffer.mark_finished();
        let eof = buffer.read(&mut empty_read).expect("eof read failed");
        assert_eq!(eof, 0);
    }

    #[test]
    fn test_precache_seek() {
        let mut buffer = SharedPrecacheBuffer::new(512);
        buffer.push_chunk(b"0123456789ABCDEF");

        // Seek from start
        buffer.seek(SeekFrom::Start(4)).unwrap();
        let mut out = [0u8; 4];
        buffer.read_exact(&mut out).unwrap();
        assert_eq!(&out, b"4567");

        // Seek current
        buffer.seek(SeekFrom::Current(2)).unwrap();
        let mut out2 = [0u8; 2];
        buffer.read_exact(&mut out2).unwrap();
        assert_eq!(&out2, b"AB");

        // Seek from end
        buffer.seek(SeekFrom::End(-4)).unwrap();
        let mut out3 = [0u8; 4];
        buffer.read_exact(&mut out3).unwrap();
        assert_eq!(&out3, b"CDEF");
    }
}
