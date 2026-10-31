//! HTTP Range-backed seekable remote file reader with LRU block cache (`TASK-138`).

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

/// Default block size fetched per HTTP `Range: bytes=start-end` request (256 KB).
pub const DEFAULT_RANGE_BLOCK_BYTES: usize = 256 * 1024;
/// Default maximum number of cached blocks in memory (32 * 256 KB = 8 MB).
pub const DEFAULT_MAX_CACHED_BLOCKS: usize = 32;

/// Abstraction over fetching a byte range `[start, end_inclusive]` from a remote resource.
pub trait RangeFetcher: Send + Sync {
    /// Fetches the inclusive byte range `[start, end_inclusive]`.
    fn fetch_range(&self, start: u64, end_inclusive: u64) -> io::Result<Vec<u8>>;
}

/// HTTP `Range` fetcher backed by `reqwest::Client` and a Tokio runtime handle.
pub struct ReqwestRangeFetcher {
    client: reqwest::Client,
    url: String,
    handle: tokio::runtime::Handle,
}

impl ReqwestRangeFetcher {
    /// Creates a new [`ReqwestRangeFetcher`] for `url`.
    pub fn new(client: reqwest::Client, url: String, handle: tokio::runtime::Handle) -> Self {
        Self {
            client,
            url,
            handle,
        }
    }
}

impl RangeFetcher for ReqwestRangeFetcher {
    fn fetch_range(&self, start: u64, end_inclusive: u64) -> io::Result<Vec<u8>> {
        let client = self.client.clone();
        let url = self.url.clone();
        let range_header = format!("bytes={start}-{end_inclusive}");
        tokio::task::block_in_place(|| {
            self.handle.block_on(async move {
                let resp = client
                    .get(&url)
                    .header(reqwest::header::RANGE, range_header)
                    .send()
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::TimedOut, e.to_string()))?;
                if !resp.status().is_success() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("HTTP range status {}", resp.status()),
                    ));
                }
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Interrupted, e.to_string()))?;
                Ok(bytes.to_vec())
            })
        })
    }
}

/// Seekable remote stream reader backed by HTTP `Range` requests and an LRU block cache (`TASK-138`).
pub struct RangeStreamReader {
    fetcher: Arc<dyn RangeFetcher>,
    total_len: u64,
    block_size: usize,
    max_blocks: usize,
    cursor: u64,
    cache: Mutex<VecDeque<(u64, Vec<u8>)>>,
}

impl RangeStreamReader {
    /// Creates a new [`RangeStreamReader`] for a remote file of length `total_len`.
    pub fn new(fetcher: Arc<dyn RangeFetcher>, total_len: u64) -> Self {
        Self::with_block_config(
            fetcher,
            total_len,
            DEFAULT_RANGE_BLOCK_BYTES,
            DEFAULT_MAX_CACHED_BLOCKS,
        )
    }

    /// Creates a [`RangeStreamReader`] with custom block size and cache capacity.
    pub fn with_block_config(
        fetcher: Arc<dyn RangeFetcher>,
        total_len: u64,
        block_size: usize,
        max_blocks: usize,
    ) -> Self {
        Self {
            fetcher,
            total_len,
            block_size: block_size.max(1024),
            max_blocks: max_blocks.max(2),
            cursor: 0,
            cache: Mutex::new(VecDeque::new()),
        }
    }

    /// Returns the total byte length advertised by `Content-Length`.
    pub fn byte_len(&self) -> Option<u64> {
        Some(self.total_len)
    }

    /// Returns `true` because this stream supports random-access HTTP `Range` seeking.
    pub fn is_seekable(&self) -> bool {
        true
    }

    fn get_or_fetch_block(&self, block_index: u64) -> io::Result<Vec<u8>> {
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(pos) = cache.iter().position(|(idx, _)| *idx == block_index) {
                let entry = cache.remove(pos).unwrap();
                let data = entry.1.clone();
                cache.push_back(entry);
                return Ok(data);
            }
        }

        let start = block_index * self.block_size as u64;
        if start >= self.total_len {
            return Ok(Vec::new());
        }
        let end_inclusive = (start + self.block_size as u64 - 1).min(self.total_len - 1);
        let data = self.fetcher.fetch_range(start, end_inclusive)?;

        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() >= self.max_blocks {
                cache.pop_front();
            }
            cache.push_back((block_index, data.clone()));
        }
        Ok(data)
    }
}

impl Read for RangeStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.cursor >= self.total_len {
            return Ok(0);
        }
        let block_index = self.cursor / self.block_size as u64;
        let block_offset = (self.cursor % self.block_size as u64) as usize;
        let block = self.get_or_fetch_block(block_index)?;
        if block_offset >= block.len() {
            return Ok(0);
        }
        let available = block.len() - block_offset;
        let to_copy = buf.len().min(available);
        buf[..to_copy].copy_from_slice(&block[block_offset..block_offset + to_copy]);
        self.cursor += to_copy as u64;
        Ok(to_copy)
    }
}

impl Seek for RangeStreamReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let next: i64 = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(delta) => self.total_len as i64 + delta,
            SeekFrom::Current(delta) => self.cursor as i64 + delta,
        };
        if next < 0 || next as u64 > self.total_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RangeStreamReader seek out of bounds",
            ));
        }
        self.cursor = next as u64;
        Ok(self.cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    struct VirtualRemoteFile {
        total_len: u64,
        requests: AtomicUsize,
    }

    impl RangeFetcher for VirtualRemoteFile {
        fn fetch_range(&self, start: u64, end_inclusive: u64) -> io::Result<Vec<u8>> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            let end = end_inclusive.min(self.total_len.saturating_sub(1));
            let len = (end - start + 1) as usize;
            let mut out = Vec::with_capacity(len);
            for offset in start..=end {
                out.push(((offset * 31 + 17) & 0xFF) as u8);
            }
            Ok(out)
        }
    }

    #[test]
    fn range_seek_to_90_percent_of_60mb_flac_starts_under_1s_and_caches_blocks() {
        let total_len: u64 = 60 * 1024 * 1024; // 60 MB remote FLAC
        let remote = Arc::new(VirtualRemoteFile {
            total_len,
            requests: AtomicUsize::new(0),
        });
        let mut reader = RangeStreamReader::new(remote.clone(), total_len);
        assert_eq!(reader.byte_len(), Some(total_len));
        assert!(reader.is_seekable());

        let started = Instant::now();
        let target_offset = (total_len * 9) / 10; // 90% = 54 MB
        let pos = reader.seek(SeekFrom::Start(target_offset)).unwrap();
        assert_eq!(pos, target_offset);

        let mut buf = vec![0u8; 4096];
        reader.read_exact(&mut buf).unwrap();
        let elapsed = started.elapsed();

        assert!(
            elapsed.as_secs_f64() < 1.0,
            "Seeking to 90% of 60 MB remote file took {elapsed:?} (>= 1s)"
        );
        assert_eq!(
            remote.requests.load(Ordering::SeqCst),
            1,
            "Seeking to 90% should fetch only the single 256 KB block containing the target offset"
        );

        // Verify byte accuracy at 90% offset
        for (i, &b) in buf.iter().enumerate() {
            let off = target_offset + i as u64;
            let expected = ((off * 31 + 17) & 0xFF) as u8;
            assert_eq!(b, expected, "mismatch at offset {off}");
        }

        // Re-reading within the same 256 KB block hits the LRU cache without issuing a second Range request
        reader.seek(SeekFrom::Start(target_offset + 512)).unwrap();
        let mut cached_buf = vec![0u8; 1024];
        reader.read_exact(&mut cached_buf).unwrap();
        assert_eq!(remote.requests.load(Ordering::SeqCst), 1);
    }
}
