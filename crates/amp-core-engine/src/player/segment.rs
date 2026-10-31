//! Plays a range of a decoded stream (TASK-232): a song inside a single-file album described by
//! a CUE sheet. The wrapper seeks to the range start, ends the stream at the range end, and keeps
//! duration, position and seeking relative to the range, so the rest of the engine (gapless
//! handoff, crossfade, end of track) treats it like any other track.

#![forbid(unsafe_code)]

use amp_domain::{Track, TrackSegment};
use amp_ports::{DecodeError, PcmStreamPort};
use std::time::Duration;

/// A decoded stream limited to one [`TrackSegment`].
pub struct SegmentedStream {
    inner: Box<dyn PcmStreamPort>,
    start: Duration,
    /// Frames left before the range end; `None` plays to the end of the source.
    remaining: Option<u64>,
    end: Option<Duration>,
}

impl SegmentedStream {
    /// Wraps `inner` and moves it to the start of `segment`.
    pub fn open(
        mut inner: Box<dyn PcmStreamPort>,
        segment: TrackSegment,
    ) -> Result<Self, DecodeError> {
        let landed = if segment.start.is_zero() {
            Duration::ZERO
        } else {
            inner.seek(segment.start)?
        };
        let mut s = Self {
            inner,
            start: segment.start,
            remaining: None,
            end: segment.end,
        };
        s.set_remaining_from(landed);
        Ok(s)
    }

    fn frames(&self, d: Duration) -> u64 {
        (d.as_secs_f64() * f64::from(self.inner.sample_rate())).round() as u64
    }

    fn set_remaining_from(&mut self, position: Duration) {
        self.remaining = self
            .end
            .map(|end| self.frames(end.saturating_sub(position)));
    }
}

impl PcmStreamPort for SegmentedStream {
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, DecodeError> {
        let limit = match self.remaining {
            Some(0) => return Ok(0),
            Some(frames) => out.len().min((frames as usize).saturating_mul(2)),
            None => out.len(),
        };
        let n = self.inner.read_interleaved(&mut out[..limit])?;
        if let Some(frames) = self.remaining.as_mut() {
            *frames = frames.saturating_sub((n / 2) as u64);
        }
        Ok(n)
    }

    fn duration(&self) -> Option<Duration> {
        match self.end {
            Some(end) => Some(end.saturating_sub(self.start)),
            None => self.inner.duration().map(|d| d.saturating_sub(self.start)),
        }
    }

    fn elapsed(&self) -> Duration {
        self.inner.elapsed().saturating_sub(self.start)
    }

    fn seek(&mut self, target: Duration) -> Result<Duration, DecodeError> {
        let mut absolute = self.start + target;
        if let Some(end) = self.end {
            absolute = absolute.min(end);
        }
        let landed = self.inner.seek(absolute)?;
        self.set_remaining_from(landed);
        Ok(landed.saturating_sub(self.start))
    }
}

/// `decoder` limited to the track's segment, if it has one.
pub fn apply_segment(
    track: &Track,
    decoder: Box<dyn PcmStreamPort>,
) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
    match track.metadata.segment {
        Some(segment) => Ok(Box::new(SegmentedStream::open(decoder, segment)?)),
        None => Ok(decoder),
    }
}
