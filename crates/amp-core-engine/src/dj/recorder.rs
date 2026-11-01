//! Recording the master mix (TASK-224). The render thread copies the finished master (after the
//! limiter) into a lock-free ring while a recording runs, and drops a block rather than wait when
//! the ring is full. A writer thread drains the ring into a WAV (32-bit float, bit for bit) or
//! FLAC (24-bit) file and tags it when the recording stops.

#![forbid(unsafe_code)]

use super::recorder_sinks::{tag_mix, FlacSink, Sink, WavSink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// Seconds of audio the ring holds while the disk is slow.
const RING_SECONDS: usize = 4;
/// How often the writer looks for new audio.
const POLL: Duration = Duration::from_millis(5);

/// File format of a recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordFormat {
    /// 32-bit float WAV: exactly what was played.
    Wav,
    /// 24-bit FLAC: lossless at 24 bits, about half the size.
    Flac,
}

impl RecordFormat {
    /// File extension without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
}

/// Tags written into a finished recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixMetadata {
    pub title: String,
    pub artist: String,
}

/// A finished recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingInfo {
    pub path: PathBuf,
    pub format: RecordFormat,
    /// Stereo frames written.
    pub frames: u64,
    /// Frames lost because the disk could not keep up.
    pub dropped_frames: u64,
}

#[derive(Debug, Default)]
struct Shared {
    on: AtomicBool,
    dropped: AtomicU64,
    written: AtomicU64,
}

/// The render thread's end: copies the master into the ring while recording.
#[derive(Debug)]
pub struct RecordTap {
    tx: rtrb::Producer<f32>,
    shared: Arc<Shared>,
}

impl RecordTap {
    /// Copies interleaved stereo `master` when recording. Never blocks, allocates or locks.
    pub fn push(&mut self, master: &[f32]) {
        if !self.shared.on.load(Ordering::Acquire) {
            return;
        }
        let n = master.len();
        match self.tx.write_chunk_uninit(n) {
            Ok(chunk) => {
                chunk.fill_from_iter(master.iter().copied());
            }
            Err(_) => {
                self.shared
                    .dropped
                    .fetch_add(n as u64 / 2, Ordering::Relaxed);
            }
        }
    }
}

type Job = JoinHandle<(rtrb::Consumer<f32>, Result<RecordingInfo, String>)>;

/// The control end: starts and stops recordings.
#[derive(Debug)]
pub struct MixRecorder {
    sample_rate: u32,
    rx: Option<rtrb::Consumer<f32>>,
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    job: Option<Job>,
}

/// A connected tap and recorder for a master at `sample_rate`.
#[must_use]
pub fn mix_recorder(sample_rate: u32) -> (RecordTap, MixRecorder) {
    let (tx, rx) = rtrb::RingBuffer::new(sample_rate as usize * 2 * RING_SECONDS);
    let shared = Arc::new(Shared::default());
    let tap = RecordTap {
        tx,
        shared: Arc::clone(&shared),
    };
    let recorder = MixRecorder {
        sample_rate,
        rx: Some(rx),
        shared,
        stop: Arc::new(AtomicBool::new(false)),
        job: None,
    };
    (tap, recorder)
}

impl MixRecorder {
    /// True while a recording runs.
    #[must_use]
    pub fn is_recording(&self) -> bool {
        self.job.is_some()
    }

    /// Seconds recorded so far.
    #[must_use]
    pub fn seconds(&self) -> f64 {
        self.shared.written.load(Ordering::Relaxed) as f64 / f64::from(self.sample_rate.max(1))
    }

    /// Frames lost so far because the disk could not keep up.
    #[must_use]
    pub fn dropped_frames(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    /// Starts recording the master into a new file at `path`.
    ///
    /// # Errors
    /// Already recording, or the file cannot be created.
    pub fn start(
        &mut self,
        path: &Path,
        format: RecordFormat,
        meta: MixMetadata,
    ) -> Result<(), String> {
        if self.job.is_some() {
            return Err("already recording".into());
        }
        let Some(mut rx) = self.rx.take() else {
            return Err("the recorder is unavailable".into());
        };
        let sink: std::io::Result<Box<dyn Sink>> = match format {
            RecordFormat::Wav => {
                WavSink::create(path, self.sample_rate).map(|s| Box::new(s) as Box<dyn Sink>)
            }
            RecordFormat::Flac => {
                FlacSink::create(path, self.sample_rate).map(|s| Box::new(s) as Box<dyn Sink>)
            }
        };
        let mut sink = match sink {
            Ok(s) => s,
            Err(e) => {
                self.rx = Some(rx);
                return Err(format!("{}: {e}", path.display()));
            }
        };
        // Anything left from before belongs to no recording.
        if let Ok(chunk) = rx.read_chunk(rx.slots()) {
            chunk.commit_all();
        }
        self.stop.store(false, Ordering::Release);
        self.shared.dropped.store(0, Ordering::Relaxed);
        self.shared.written.store(0, Ordering::Relaxed);
        let (stop, shared, path) = (
            Arc::clone(&self.stop),
            Arc::clone(&self.shared),
            path.to_path_buf(),
        );
        let spawned = std::thread::Builder::new()
            .name("multiplay-mix-recorder".into())
            .spawn(move || {
                let result = drain(&mut rx, sink.as_mut(), &stop, &shared)
                    .and_then(|frames| {
                        sink.finish()?;
                        Ok(frames)
                    })
                    .map_err(|e| e.to_string())
                    .map(|frames| {
                        if let Err(e) = tag_mix(&path, &meta) {
                            tracing::warn!("the recording could not be tagged: {e}");
                        }
                        RecordingInfo {
                            path,
                            format,
                            frames,
                            dropped_frames: shared.dropped.load(Ordering::Relaxed),
                        }
                    });
                (rx, result)
            });
        match spawned {
            Ok(job) => {
                self.job = Some(job);
                self.shared.on.store(true, Ordering::Release);
                Ok(())
            }
            Err(e) => Err(format!("the recorder could not start: {e}")),
        }
    }

    /// Stops the recording, finishes and tags the file.
    ///
    /// # Errors
    /// Nothing is recording, or writing the file failed.
    pub fn stop(&mut self) -> Result<RecordingInfo, String> {
        let job = self.job.take().ok_or("not recording")?;
        self.shared.on.store(false, Ordering::Release);
        self.stop.store(true, Ordering::Release);
        let (rx, result) = job
            .join()
            .map_err(|_| "the recorder stopped unexpectedly".to_string())?;
        self.rx = Some(rx);
        result
    }
}

impl Drop for MixRecorder {
    fn drop(&mut self) {
        if self.job.is_some() {
            let _ = self.stop();
        }
    }
}

/// Writes what arrives until stopped; then takes what is left, waiting one poll for a block the
/// render thread may still be copying.
fn drain(
    rx: &mut rtrb::Consumer<f32>,
    sink: &mut dyn Sink,
    stop: &AtomicBool,
    shared: &Shared,
) -> std::io::Result<u64> {
    let mut frames = 0u64;
    let mut quiet_after_stop = 0;
    loop {
        let n = rx.slots() / 2 * 2;
        if n > 0 {
            let chunk = rx
                .read_chunk(n)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let (a, b) = chunk.as_slices();
            sink.write(a)?;
            sink.write(b)?;
            chunk.commit_all();
            frames += n as u64 / 2;
            shared.written.store(frames, Ordering::Relaxed);
            quiet_after_stop = 0;
            continue;
        }
        if stop.load(Ordering::Acquire) {
            quiet_after_stop += 1;
            if quiet_after_stop > 1 {
                return Ok(frames);
            }
        }
        std::thread::sleep(POLL);
    }
}
