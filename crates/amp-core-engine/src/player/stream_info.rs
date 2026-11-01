//! What is really playing (TASK-170): the decoder's sample rate, the output rate, and whether
//! the last audio went out bit-perfect. Written by the worker for every chunk (two atomics),
//! read by the front ends for the faceplate.

#![forbid(unsafe_code)]

use super::PlayerEngine;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The format of the stream that is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInfo {
    /// The decoder's sample rate in Hz.
    pub source_rate: u32,
    /// The output device's sample rate in Hz.
    pub output_rate: u32,
    /// No resampling, no volume change, no EQ or limiter: the samples reach the output unchanged.
    pub bit_perfect: bool,
}

/// Shared between the worker (writer) and the UI (reader).
#[derive(Debug, Default)]
pub struct StreamFormat {
    source_rate: AtomicU32,
    bit_perfect: AtomicBool,
}

impl StreamFormat {
    /// Records the chunk that just went out.
    pub(crate) fn record(&self, source_rate: u32, bit_perfect: bool) {
        self.source_rate.store(source_rate, Ordering::Relaxed);
        self.bit_perfect.store(bit_perfect, Ordering::Relaxed);
    }

    /// Nothing is decoding any more.
    pub(crate) fn clear(&self) {
        self.source_rate.store(0, Ordering::Relaxed);
    }
}

impl PlayerEngine {
    /// The stream that is playing, or `None` when nothing is decoding.
    #[must_use]
    pub fn stream_info(&self) -> Option<StreamInfo> {
        let source_rate = self.stream_format.source_rate.load(Ordering::Relaxed);
        (source_rate > 0).then(|| StreamInfo {
            source_rate,
            output_rate: self.sample_rate(),
            bit_perfect: self.stream_format.bit_perfect.load(Ordering::Relaxed),
        })
    }
}
