//! Universal MediaSource adapter for Symphonia.
//! Adapts any `Read + Seek + Send + Sync + 'static` stream to Symphonia's `MediaSource`.

#![forbid(unsafe_code)]

use std::io::{Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;

/// MediaSource wrapper adapting any `Read + Seek + Send + Sync + 'static` stream to Symphonia's interface.
pub struct GenericMediaSource<S> {
    inner: S,
    seekable: bool,
    byte_len: Option<u64>,
}

impl<S: Read + Seek + Send + Sync> GenericMediaSource<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            seekable: true,
            byte_len: None,
        }
    }

    pub fn with_byte_len(inner: S, byte_len: Option<u64>) -> Self {
        Self {
            inner,
            seekable: true,
            byte_len,
        }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Read + Seek + Send + Sync> Read for GenericMediaSource<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
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
