//! Runs a [`DjEngine`] on its own thread, keeping each output topped up to a small lead (about
//! 50 ms) so faders, cue and crossfader react quickly, unlike the normal player's deep buffer.

#![forbid(unsafe_code)]

use super::render::DjEngine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// Frames rendered per step.
const STEP: usize = 256;

/// Where the runtime writes audio: an interleaved stereo queue drained by a sound card.
pub trait AudioOut: Send {
    /// Samples queued and not yet played.
    fn queued(&self) -> usize;
    /// Appends samples; returns how many fit.
    fn write(&mut self, samples: &[f32]) -> usize;
}

impl AudioOut for rtrb::Producer<f32> {
    fn queued(&self) -> usize {
        self.buffer().capacity() - self.slots()
    }

    fn write(&mut self, samples: &[f32]) -> usize {
        let n = samples.len().min(self.slots());
        match self.write_chunk_uninit(n) {
            Ok(chunk) => chunk.fill_from_iter(samples[..n].iter().copied()),
            Err(_) => 0,
        }
    }
}

/// A running engine thread.
#[derive(Debug)]
pub struct DjRuntime {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DjRuntime {
    /// Starts rendering `engine` into `master` (and `cue`, when there is a cue output), keeping
    /// about `lead_frames` frames queued.
    pub fn start(
        mut engine: DjEngine,
        mut master: Box<dyn AudioOut>,
        mut cue: Option<Box<dyn AudioOut>>,
        lead_frames: usize,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("multiplay-dj-render".into())
            .spawn(move || {
                let mut m = vec![0.0f32; STEP * 2];
                let mut c = vec![0.0f32; STEP * 2];
                while !flag.load(Ordering::Relaxed) {
                    if master.queued() / 2 >= lead_frames {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    engine.render(&mut m, &mut c);
                    master.write(&m);
                    if let Some(out) = cue.as_mut() {
                        out.write(&c);
                    }
                }
            })
            .ok();
        Self { stop, thread }
    }

    /// Stops the thread and waits for it.
    pub fn stop(mut self) {
        self.halt();
    }

    fn halt(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for DjRuntime {
    fn drop(&mut self) {
        self.halt();
    }
}
