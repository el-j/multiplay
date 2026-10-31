//! In-memory and bounded disk-spill stream buffer for concurrent audio precaching.
//! Re-exports [`SharedPrecacheBuffer`] and [`PrecacheStreamReader`] from `amp-ports`
//! and provides [`DiskSpillCache`] for size-capped LRU cache management (`TASK-136`).

#![forbid(unsafe_code)]

pub use amp_ports::{
    PrecacheStreamReader, SharedPrecacheBuffer, DEFAULT_BUFFER_WATERMARK_BYTES,
    DEFAULT_LIVE_WINDOW_BYTES,
};

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SPILL_ID: AtomicU64 = AtomicU64::new(1);

/// LRU-evicted, size-capped disk cache directory manager for finite stream spill files (`TASK-136`).
#[derive(Debug)]
pub struct DiskSpillCache {
    cache_dir: PathBuf,
    max_total_bytes: u64,
    entries: VecDeque<(PathBuf, u64)>,
}

impl DiskSpillCache {
    /// Creates a new [`DiskSpillCache`] rooted at `cache_dir` with total disk cap `max_total_bytes`.
    pub fn new(cache_dir: impl Into<PathBuf>, max_total_bytes: u64) -> std::io::Result<Self> {
        let cache_dir = cache_dir.into();
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            cache_dir,
            max_total_bytes: max_total_bytes.max(64 * 1024),
            entries: VecDeque::new(),
        })
    }

    /// Returns the root cache directory.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Allocates a new spill file path and evicts oldest spill files if the directory exceeds `max_total_bytes`.
    pub fn allocate_spill_path(&mut self, estimated_bytes: u64) -> PathBuf {
        self.evict_to_fit(estimated_bytes);
        let id = NEXT_SPILL_ID.fetch_add(1, Ordering::Relaxed);
        let path = self.cache_dir.join(format!("stream-spill-{id}.bin"));
        self.entries.push_back((path.clone(), estimated_bytes));
        path
    }

    /// Refreshes disk usage from actual file metadata and evicts oldest entries until usage + `incoming` fits.
    pub fn evict_to_fit(&mut self, incoming: u64) {
        for (path, size) in &mut self.entries {
            if let Ok(meta) = std::fs::metadata(&*path) {
                *size = meta.len();
            }
        }
        let mut total: u64 = self.entries.iter().map(|(_, s)| *s).sum();
        while total.saturating_add(incoming) > self.max_total_bytes && !self.entries.is_empty() {
            if let Some((old_path, old_size)) = self.entries.pop_front() {
                let _ = std::fs::remove_file(&old_path);
                total = total.saturating_sub(old_size);
            }
        }
    }

    /// Returns the total tracked bytes across active spill files on disk.
    pub fn total_bytes_on_disk(&self) -> u64 {
        self.entries
            .iter()
            .filter_map(|(p, _)| std::fs::metadata(p).ok().map(|m| m.len()))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::{Read, Seek, SeekFrom};
    use std::time::Duration;

    proptest! {
        #[test]
        fn test_precache_arbitrary_chunk_invariants(
            chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..64), 1..16)
        ) {
            let total_bytes: usize = chunks.iter().map(|c| c.len()).sum();
            let mut buffer = SharedPrecacheBuffer::new(total_bytes + 1024);

            for chunk in &chunks {
                buffer.push_chunk(chunk);
            }
            buffer.mark_finished();

            prop_assert_eq!(buffer.total_bytes_cached(), total_bytes);

            let mut assembled = Vec::with_capacity(total_bytes);
            let mut read_buf = [0u8; 32];
            loop {
                let n = buffer.read(&mut read_buf).unwrap();
                if n == 0 {
                    break;
                }
                assembled.extend_from_slice(&read_buf[..n]);
            }

            let expected: Vec<u8> = chunks.into_iter().flatten().collect();
            prop_assert_eq!(assembled, expected);
        }
    }

    #[test]
    fn bounded_buffer_live_radio_caps_memory_growth() {
        // Simulate 20 MB of continuous radio stream through a 512 KB sliding window
        let window_bytes = 512 * 1024;
        let mut live = SharedPrecacheBuffer::new_live_ring(window_bytes, Duration::from_secs(1));
        let chunk = vec![0x5Au8; 32 * 1024];
        let mut read_sink = vec![0u8; 16 * 1024];

        for _ in 0..640 {
            live.push_chunk(&chunk);
            let _ = live.read(&mut read_sink).unwrap();
            let _ = live.read(&mut read_sink).unwrap();
            assert!(
                live.ram_bytes_resident() <= window_bytes,
                "live radio RAM resident {} exceeded window {}",
                live.ram_bytes_resident(),
                window_bytes
            );
        }
        assert_eq!(live.total_bytes_cached(), 640 * chunk.len());
        assert!(live.ram_bytes_resident() <= window_bytes);
    }

    #[test]
    fn bounded_buffer_spills_to_disk_and_lru_evicts() {
        let temp_dir = std::env::temp_dir().join(format!("multiplay-spill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let mut cache = DiskSpillCache::new(&temp_dir, 200 * 1024).unwrap();

        let spill_path_1 = cache.allocate_spill_path(120 * 1024);
        let ram_limit = 32 * 1024;
        let mut buf1 = SharedPrecacheBuffer::new(ram_limit)
            .with_disk_spill(ram_limit, spill_path_1.clone())
            .unwrap();

        let payload: Vec<u8> = (0..(120 * 1024)).map(|i| (i % 251) as u8).collect();
        for piece in payload.chunks(8192) {
            buf1.push_chunk(piece);
        }
        buf1.mark_finished();

        assert!(
            buf1.ram_bytes_resident() <= ram_limit,
            "RAM resident {} must not exceed ram_limit {}",
            buf1.ram_bytes_resident(),
            ram_limit
        );
        assert!(spill_path_1.exists(), "spill file should exist on disk");

        // Read across the RAM/disk boundary and seek back and forth
        let mut out = Vec::new();
        buf1.read_to_end(&mut out).unwrap();
        assert_eq!(out, payload);

        buf1.seek(SeekFrom::Start(50_000)).unwrap();
        let mut tail = vec![0u8; 1024];
        buf1.read_exact(&mut tail).unwrap();
        assert_eq!(&tail[..], &payload[50_000..51_024]);

        // Allocating a second 120 KB spill file exceeds the 200 KB cap and evicts spill_path_1
        let spill_path_2 = cache.allocate_spill_path(120 * 1024);
        assert!(
            !spill_path_1.exists(),
            "LRU eviction should remove the oldest spill file when exceeding cap"
        );
        assert_ne!(spill_path_1, spill_path_2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn bounded_buffer_dropping_stream_cancels_50_network_producers() {
        let mut buffers = Vec::with_capacity(50);
        for _ in 0..50 {
            let buf = SharedPrecacheBuffer::new(4096);
            let reader = buf.clone().into_reader();
            buffers.push(buf);
            // Simulate immediate track skip by dropping reader
            drop(reader);
        }
        for (idx, buf) in buffers.iter().enumerate() {
            assert!(
                buf.is_cancelled(),
                "producer {idx} must be cancelled when reader drops"
            );
        }
    }
}
