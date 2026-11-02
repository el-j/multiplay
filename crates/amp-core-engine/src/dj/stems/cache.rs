//! In-memory and offline caching for separated stems.

#![forbid(unsafe_code)]

use super::separator::SeparatedStems;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// In-memory cache for track stems with bounded capacity.
#[derive(Debug)]
pub struct StemCache {
    capacity: usize,
    entries: HashMap<String, Arc<SeparatedStems>>,
    order: VecDeque<String>,
}

impl StemCache {
    /// Creates a new cache with the specified capacity of tracks.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    /// Retrieves pre-separated stems for a track by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Arc<SeparatedStems>> {
        self.entries.get(key).cloned()
    }

    /// Stores separated stems for a track by key, evicting the oldest entry
    /// if capacity is exceeded.
    pub fn put(&mut self, key: &str, stems: Arc<SeparatedStems>) {
        if self.entries.contains_key(key) {
            self.entries.insert(key.to_string(), stems);
            return;
        }

        while self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            } else {
                break;
            }
        }

        self.entries.insert(key.to_string(), stems);
        self.order.push_back(key.to_string());
    }

    /// Returns the number of cached tracks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}
