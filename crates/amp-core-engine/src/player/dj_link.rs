//! Hand-over point between a UI DJ session and the `PlayerEngine` audio worker.
//!
//! The session puts the DJ engine and requests here; the worker picks them up between blocks.
//! Each request is a rare event (entering or leaving DJ mode), so a mutex per request is fine.
//! Audio itself never waits on these locks.

#![forbid(unsafe_code)]

use crate::dj::engine::{DjEngine, DjStatus, TrackBuffer};
use amp_domain::Track;
use rtrb::Producer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// What the DJ session asks the `PlayerEngine` worker to do.
#[derive(Debug)]
pub enum DjRequest {
    /// Take over from normal playback. Deck A's track is decoding into `deck_a`; the worker
    /// switches once it is decoded past the playing position.
    Enter {
        /// Deck A's copy of the playing track, if something is loaded.
        deck_a: Option<Arc<TrackBuffer>>,
    },
    /// Hand the on-air deck back to normal playback at its position.
    Leave {
        /// The on-air deck's track, if any.
        track: Option<Box<Track>>,
        /// The on-air deck.
        deck: usize,
        /// Where to read that deck's position at the moment of the switch.
        status: Arc<DjStatus>,
        /// Whether it was playing.
        playing: bool,
    },
}

/// Shared between the DJ session and the playback worker.
#[derive(Debug, Default)]
pub struct DjLink {
    engine: Mutex<Option<DjEngine>>,
    request: Mutex<Option<DjRequest>>,
    phones: Mutex<Option<Producer<f32>>>,
    active: AtomicBool,
}

impl DjLink {
    /// Installs the engine; the worker takes it on the next DJ request.
    pub fn install_engine(&self, engine: DjEngine) {
        if let Ok(mut slot) = self.engine.lock() {
            *slot = Some(engine);
        }
    }

    pub(crate) fn take_engine(&self) -> Option<DjEngine> {
        self.engine.lock().ok()?.take()
    }

    /// Queues a request (replacing an unhandled one).
    pub fn request(&self, r: DjRequest) {
        if let Ok(mut slot) = self.request.lock() {
            *slot = Some(r);
        }
    }

    pub(crate) fn take_request(&self) -> Option<DjRequest> {
        self.request.try_lock().ok()?.take()
    }

    /// Sets (or removes) the headphone device's queue for second-device cue routing.
    pub fn set_phones(&self, p: Option<Producer<f32>>) {
        if let Ok(mut slot) = self.phones.lock() {
            *slot = p;
        }
    }

    pub(crate) fn phones(&self) -> &Mutex<Option<Producer<f32>>> {
        &self.phones
    }

    /// True while the DJ engine, not normal playback, produces the audio.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(crate) fn set_active(&self, on: bool) {
        self.active.store(on, Ordering::Release);
    }
}
