//! Transport actions shared by the buttons and the keyboard (TASK-175): Winamp's Z X C V B, J
//! and Ctrl+L, plus space, arrows for seek and volume, and seek-on-release for the scrubber.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use crate::docking::WindowId;
use eframe::egui::{self, Key, Modifiers};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Something the transport can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportAction {
    Previous,
    /// Play the selected track (restart it if it is playing).
    Play,
    PlayPause,
    Stop,
    Next,
    /// Seek by this many seconds.
    SeekBy(i32),
    /// Change the volume by this many percentage points.
    VolumeBy(i32),
    /// Winamp's "jump to file": the search window.
    JumpToFile,
    /// Winamp's "open location": the search window with the cursor in the link field.
    OpenLocation,
}

/// The action for a key press, if any. Plain keys only, apart from Ctrl/Cmd+L: other command
/// shortcuts (copy, paste, settings) belong to the app.
#[must_use]
pub fn key_action(key: Key, modifiers: Modifiers) -> Option<TransportAction> {
    use TransportAction as A;
    if modifiers.command {
        return (key == Key::L).then_some(A::OpenLocation);
    }
    if modifiers.alt || modifiers.ctrl {
        return None;
    }
    Some(match key {
        Key::Z => A::Previous,
        Key::X => A::Play,
        Key::C | Key::Space => A::PlayPause,
        Key::V => A::Stop,
        Key::B => A::Next,
        Key::ArrowLeft => A::SeekBy(-5),
        Key::ArrowRight => A::SeekBy(5),
        Key::ArrowUp => A::VolumeBy(5),
        Key::ArrowDown => A::VolumeBy(-5),
        Key::J => A::JumpToFile,
        _ => return None,
    })
}

/// Runs the transport keys pressed this frame, unless a text field has the keyboard or DJ mode
/// (which has its own key map) is on.
pub fn handle_keys(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() || app.dj_active() {
        return;
    }
    let actions: Vec<TransportAction> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } => key_action(*key, *modifiers).filter(|a| {
                    !repeat
                        || matches!(a, TransportAction::SeekBy(_) | TransportAction::VolumeBy(_))
                }),
                _ => None,
            })
            .collect()
    });
    for action in actions {
        app.apply_transport(action);
    }
}

impl MultiplayGuiApp {
    /// Plays playlist entry `idx` and queues the one after it for gapless playback.
    pub fn play_index(&mut self, idx: usize) {
        let Some(track) = self.playlist.get(idx).cloned() else {
            return;
        };
        self.selected_track_idx = idx;
        self.audio_driver.play(track);
        let upcoming = self.playlist.get(idx + 1).cloned();
        self.audio_driver.set_next_track(upcoming);
    }

    /// Does what a transport button or key asks.
    pub fn apply_transport(&mut self, action: TransportAction) {
        use TransportAction as A;
        let playing = self.audio_driver.is_playing.load(Ordering::Relaxed);
        match action {
            A::Previous if self.selected_track_idx > 0 => {
                self.play_index(self.selected_track_idx - 1)
            }
            A::Next if self.selected_track_idx + 1 < self.playlist.len() => {
                self.play_index(self.selected_track_idx + 1);
            }
            A::Previous | A::Next => {}
            A::Play => self.play_index(self.selected_track_idx),
            A::PlayPause if playing => self.audio_driver.toggle_play_pause(),
            A::PlayPause => {
                let paused = self
                    .audio_driver
                    .current_track
                    .lock()
                    .is_ok_and(|t| t.is_some());
                if paused {
                    self.audio_driver.toggle_play_pause();
                } else {
                    self.play_index(self.selected_track_idx);
                }
            }
            A::Stop => self.audio_driver.stop(),
            A::SeekBy(secs) => {
                let elapsed = *self.audio_driver.elapsed.lock().unwrap();
                let target = if secs < 0 {
                    elapsed.saturating_sub(Duration::from_secs(secs.unsigned_abs().into()))
                } else {
                    elapsed + Duration::from_secs(secs.unsigned_abs().into())
                };
                let total = *self.audio_driver.total_duration.lock().unwrap();
                self.audio_driver
                    .seek(total.map_or(target, |t| target.min(t)));
            }
            A::VolumeBy(points) => {
                self.volume = (self.volume + points as f32 / 100.0).clamp(0.0, 1.0);
                self.audio_driver.set_volume(self.volume);
            }
            A::JumpToFile => self.dock_manager.set_visible(WindowId::Search, true),
            A::OpenLocation => {
                self.dock_manager.set_visible(WindowId::Search, true);
                self.focus_location_field = true;
            }
        }
    }
}

/// The scrubber: dragging moves the knob, releasing seeks once (seeking on every drag step
/// stuttered and flooded the decoder with seeks).
#[derive(Debug, Default, Clone, Copy)]
pub struct Scrub {
    dragging: Option<f32>,
}

impl Scrub {
    /// The knob is being dragged to `at` (0..=1). Never seeks.
    pub fn drag_to(&mut self, at: f32) -> Option<f32> {
        self.dragging = Some(at);
        None
    }

    /// The drag ended: seek to where the knob was let go.
    pub fn release(&mut self) -> Option<f32> {
        self.dragging.take()
    }

    /// A click without a drag seeks straight away.
    pub fn click(&mut self, at: f32) -> Option<f32> {
        self.dragging = None;
        Some(at)
    }

    /// Where to draw the knob: the drag target while dragging, otherwise `playing`.
    #[must_use]
    pub fn shown(&self, playing: f32) -> f32 {
        self.dragging.unwrap_or(playing)
    }
}
