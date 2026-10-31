//! A track can be a range of a file (CUE sheets, TASK-232): the stream starts at the range's
//! start, ends at its end, and reports duration, position and seeks relative to the range.

#![forbid(unsafe_code)]

use amp_core_engine::player::segment::SegmentedStream;
use amp_domain::TrackSegment;
use amp_ports::{DecodeError, PcmStreamPort};
use std::time::Duration;

const RATE: u32 = 1000;

/// A 10 s stream whose sample values count frames, so every read shows where it came from.
struct Counter {
    frame: u64,
    total: u64,
}

impl PcmStreamPort for Counter {
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, DecodeError> {
        let mut n = 0;
        while n + 1 < out.len() && self.frame < self.total {
            out[n] = self.frame as f32;
            out[n + 1] = self.frame as f32;
            self.frame += 1;
            n += 2;
        }
        Ok(n)
    }
    fn duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(self.total))
    }
    fn elapsed(&self) -> Duration {
        Duration::from_millis(self.frame)
    }
    fn seek(&mut self, target: Duration) -> Result<Duration, DecodeError> {
        self.frame = (target.as_millis() as u64).min(self.total);
        Ok(self.elapsed())
    }
}

fn counter() -> Box<dyn PcmStreamPort> {
    Box::new(Counter {
        frame: 0,
        total: 10_000,
    })
}

fn segment(start_ms: u64, end_ms: Option<u64>) -> TrackSegment {
    TrackSegment {
        start: Duration::from_millis(start_ms),
        end: end_ms.map(Duration::from_millis),
    }
}

fn frames(stream: &mut dyn PcmStreamPort) -> Vec<f32> {
    let mut all = Vec::new();
    let mut buf = [0.0f32; 333];
    loop {
        let n = stream.read_interleaved(&mut buf).unwrap();
        if n == 0 {
            return all;
        }
        all.extend(buf[..n].iter().step_by(2));
    }
}

#[test]
fn plays_exactly_the_range() {
    let mut s = SegmentedStream::open(counter(), segment(2_000, Some(5_000))).unwrap();
    let got = frames(&mut s);
    assert_eq!(got.len(), 3_000);
    assert_eq!(got.first(), Some(&2_000.0));
    assert_eq!(got.last(), Some(&4_999.0));
    assert_eq!(s.duration(), Some(Duration::from_secs(3)));
}

#[test]
fn an_open_ended_range_runs_to_the_end_of_the_file() {
    let mut s = SegmentedStream::open(counter(), segment(7_000, None)).unwrap();
    assert_eq!(s.duration(), Some(Duration::from_secs(3)));
    let got = frames(&mut s);
    assert_eq!((got.len(), got[0]), (3_000, 7_000.0));
}

#[test]
fn position_and_seeking_are_relative_to_the_range() {
    let mut s = SegmentedStream::open(counter(), segment(2_000, Some(5_000))).unwrap();
    assert_eq!(s.elapsed(), Duration::ZERO);
    let landed = s.seek(Duration::from_secs(1)).unwrap();
    assert_eq!(landed, Duration::from_secs(1));
    let mut buf = [0.0f32; 2];
    s.read_interleaved(&mut buf).unwrap();
    assert_eq!(buf[0], 3_000.0, "1 s into the range is 3 s into the file");
    // Seeking past the range end clamps to its end: nothing more to play.
    s.seek(Duration::from_secs(60)).unwrap();
    assert_eq!(s.read_interleaved(&mut buf).unwrap(), 0);
}
