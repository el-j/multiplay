//! N-deck DJ engine (ADR-003): decks decode into memory on loader threads, a render thread mixes
//! them at a variable rate into a master and a cue bus, and the UI steers it through lock-free
//! parameters and a command queue.

#![forbid(unsafe_code)]

pub mod bus;
pub mod deck_commands;
mod keylock;
pub mod limiter;
pub mod loops;
pub mod mixer_params;
pub mod params;
mod player;
pub mod render;
pub mod runtime;
pub mod status;
pub mod strip;
pub mod sync;
pub mod track_buffer;

pub use bus::{route, CueMix, CueRouting};
pub use deck_commands::DeckCommand;
pub use limiter::LOOKAHEAD as LATENCY_FRAMES;
pub use loops::HOT_CUES;
pub use mixer_params::{curve_gains, Band, ChannelControls, CrossfaderCurve, MasterControls};
pub use params::{
    AtomicF32, AtomicF64, CrossfaderSide, DeckParams, DjParams, CROSSFADER_LEFT, CROSSFADER_RIGHT,
    MAX_DECKS, MAX_PITCH_PERCENT,
};
pub use render::{crossfader_gains, DjCommand, DjControl, DjEngine};
pub use runtime::{AudioOut, DjOutputs, DjRuntime};
pub use status::{DeckStatus, DjStatus};
pub use sync::PitchRange;
pub use track_buffer::{spawn_loader, TrackBuffer, TrackWriter, CHUNK_FRAMES};
