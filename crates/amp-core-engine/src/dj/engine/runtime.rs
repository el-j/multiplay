//! Runs a [`DjEngine`] on its own thread, keeping each output topped up to a small lead (about
//! 50 ms) so faders, cue and crossfader react quickly, unlike the normal player's deep buffer.

#![forbid(unsafe_code)]

use super::bus::{route, CueRouting};
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

/// The sound card queues the runtime feeds.
pub struct DjOutputs {
    /// Main device queue, interleaved with `main_channels` channels.
    pub main: Box<dyn AudioOut>,
    /// Channels of the main device (2 for stereo, 4 for a DJ interface).
    pub main_channels: usize,
    /// Headphone device queue (stereo), for [`CueRouting::SecondDevice`].
    pub phones: Option<Box<dyn AudioOut>>,
}

impl std::fmt::Debug for DjOutputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DjOutputs")
            .field("main_channels", &self.main_channels)
            .field("phones", &self.phones.is_some())
            .finish()
    }
}

/// A running engine thread.
#[derive(Debug)]
pub struct DjRuntime {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DjRuntime {
    /// Starts rendering `engine` into `out`, keeping about `lead_frames` frames queued on the
    /// main device. The headphone mix is routed as `DjParams::cue_routing` says; a second-device
    /// routing without a headphone queue plays master only.
    pub fn start(mut engine: DjEngine, mut out: DjOutputs, lead_frames: usize) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let params = engine.params();
        let ch = out.main_channels.max(1);
        let thread = std::thread::Builder::new()
            .name("multiplay-dj-render".into())
            .spawn(move || {
                let mut master = vec![0.0f32; STEP * 2];
                let mut cue = vec![0.0f32; STEP * 2];
                let mut main = vec![0.0f32; STEP * ch];
                let mut phones = vec![0.0f32; STEP * 2];
                while !flag.load(Ordering::Relaxed) {
                    if out.main.queued() / ch >= lead_frames {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    engine.render(&mut master, &mut cue);
                    let routing = match params.cue_routing() {
                        CueRouting::SecondDevice if out.phones.is_none() => CueRouting::Off,
                        r => r,
                    };
                    route(
                        routing,
                        &master,
                        &cue,
                        params.cue_mix(),
                        ch,
                        &mut main,
                        &mut phones,
                    );
                    out.main.write(&main);
                    if let (CueRouting::SecondDevice, Some(p)) = (routing, out.phones.as_mut()) {
                        // The headphone device runs on its own clock; drop a block rather than
                        // let its queue (and so its latency) grow.
                        if p.queued() / 2 < lead_frames * 2 {
                            p.write(&phones);
                        }
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
