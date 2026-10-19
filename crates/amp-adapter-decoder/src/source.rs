//! Universal MediaSource adapter for Symphonia.
//! Adapts any `Read + Seek + Send + Sync + 'static` stream to Symphonia's `MediaSource`.

#![forbid(unsafe_code)]

use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::time::{Duration, Instant};
use symphonia::core::io::MediaSource;

/// How long a read waits for a stalled progressive stream before reporting `WouldBlock`.
pub const DEFAULT_STALL_TIMEOUT: Duration = Duration::from_secs(5);
const UNDERRUN_POLL: Duration = Duration::from_millis(2);

/// MediaSource wrapper adapting any `Read + Seek + Send + Sync + 'static` stream to Symphonia's interface.
///
/// A `WouldBlock` from the inner stream (network underrun) is waited out here instead of being
/// passed on: Symphonia drops the bytes it already copied for a packet when a read fails part-way,
/// so surfacing the error would lose audio. Only a stall longer than the timeout is reported.
pub struct GenericMediaSource<S> {
    inner: S,
    seekable: bool,
    byte_len: Option<u64>,
    stall_timeout: Duration,
}

impl<S: Read + Seek + Send + Sync> GenericMediaSource<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            seekable: true,
            byte_len: None,
            stall_timeout: DEFAULT_STALL_TIMEOUT,
        }
    }

    pub fn with_byte_len(inner: S, byte_len: Option<u64>) -> Self {
        Self {
            inner,
            seekable: true,
            byte_len,
            stall_timeout: DEFAULT_STALL_TIMEOUT,
        }
    }

    /// Overrides how long a read waits for a stalled stream (see [`DEFAULT_STALL_TIMEOUT`]).
    #[must_use]
    pub fn with_stall_timeout(mut self, stall_timeout: Duration) -> Self {
        self.stall_timeout = stall_timeout;
        self
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Read + Seek + Send + Sync> Read for GenericMediaSource<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let started = Instant::now();
        loop {
            match self.inner.read(buf) {
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if started.elapsed() >= self.stall_timeout {
                        return Err(e);
                    }
                    std::thread::sleep(UNDERRUN_POLL);
                }
                other => return other,
            }
        }
    }
}

impl<S: Read + Seek + Send + Sync> Seek for GenericMediaSource<S> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<S: Read + Seek + Send + Sync + 'static> MediaSource for GenericMediaSource<S> {
    fn is_seekable(&self) -> bool {
        self.seekable
    }

    fn byte_len(&self) -> Option<u64> {
        self.byte_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_generic_media_source_read_and_seek() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let cursor = Cursor::new(data);
        let mut source = GenericMediaSource::new(cursor);

        assert!(source.is_seekable());
        assert_eq!(source.byte_len(), None);

        let mut buf = [0u8; 4];
        let n = source.read(&mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(&buf, &[1, 2, 3, 4]);

        let pos = source.seek(SeekFrom::Start(2)).unwrap();
        assert_eq!(pos, 2);

        let n = source.read(&mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(&buf, &[3, 4, 5, 6]);
    }
}
