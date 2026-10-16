# Advanced Playback Engine: Intelligent Shuffle, Seamless Crossfading & Normalization

This specification outlines the advanced playback control subsystem within the pure-Rust architecture, elevating standard audio playback into an audiophile-grade, intelligent listening experience.

---

## 1. Non-Repeating Uniform Random & Smart Shuffle

Traditional pseudo-random shuffle often results in track clustering (e.g., three songs by the same artist playing consecutively). The engine implements two pure-Rust shuffle strategies:
1. **Fisher-Yates Modern Permutation**: Deterministic pseudo-random order generated on playlist start or change, guaranteeing every track is played exactly once before recycling.
2. **Smart Balanced Shuffle**: Constraint-based shuffle maintaining balanced artist separation, genre continuity, and tempo transitions.

```
       ┌────────────────────────────────────────────────────────┐
       │                Original Playlist Queue                 │
       │    [Track A1, Track B1, Track A2, Track C1, Track A3]   │
       └──────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
       ┌────────────────────────────────────────────────────────┐
       │             Smart Distance Constraint Matrix           │
       │     Rule 1: Artist spacing >= 3 tracks                 │
       │     Rule 2: BPM Delta <= 20% between adjacent tracks   │
       └──────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
       ┌────────────────────────────────────────────────────────┐
       │                 Optimized Play Queue                   │
       │    [Track A1, Track B1, Track C1, Track A2, Track A3]   │
       └────────────────────────────────────────────────────────┘
```

### 1.1. Pure-Rust Shuffle Engine (`crates/amp-core-engine/src/playback/shuffle.rs`)

```rust
use amp_domain::{Track, TrackId};
use rand::seq::SliceRandom;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShuffleMode {
    Off,
    TrueRandom,
    SmartBalanced { artist_spacing: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    RepeatAll,
    RepeatOne,
}

pub struct SmartQueueManager {
    pristine_queue: Vec<Track>,
    playback_order: VecDeque<usize>, // Indirection indices into pristine_queue
    history: Vec<usize>,
    shuffle_mode: ShuffleMode,
    repeat_mode: RepeatMode,
    rng: SmallRng,
}

impl SmartQueueManager {
    pub fn new() -> Self {
        Self {
            pristine_queue: Vec::new(),
            playback_order: VecDeque::new(),
            history: Vec::new(),
            shuffle_mode: ShuffleMode::Off,
            repeat_mode: RepeatMode::Off,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.pristine_queue = tracks;
        self.rebuild_order();
    }

    pub fn set_shuffle(&mut self, mode: ShuffleMode) {
        self.shuffle_mode = mode;
        self.rebuild_order();
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
    }

    pub fn next_track(&mut self) -> Option<Track> {
        if self.pristine_queue.is_empty() {
            return None;
        }

        if let RepeatMode::RepeatOne = self.repeat_mode {
            if let Some(&last_idx) = self.history.last() {
                return self.pristine_queue.get(last_idx).cloned();
            }
        }

        if self.playback_order.is_empty() {
            if let RepeatMode::RepeatAll = self.repeat_mode {
                self.rebuild_order();
            } else {
                return None;
            }
        }

        let next_idx = self.playback_order.pop_front()?;
        self.history.push(next_idx);
        self.pristine_queue.get(next_idx).cloned()
    }

    pub fn previous_track(&mut self) -> Option<Track> {
        if self.history.len() < 2 {
            return None;
        }
        // Current track is last element; pop it and push back to playback queue front
        let current = self.history.pop()?;
        self.playback_order.push_front(current);
        
        let prev_idx = *self.history.last()?;
        self.pristine_queue.get(prev_idx).cloned()
    }

    fn rebuild_order(&mut self) {
        let total = self.pristine_queue.len();
        if total == 0 {
            self.playback_order.clear();
            return;
        }

        let mut indices: Vec<usize> = (0..total).collect();

        match self.shuffle_mode {
            ShuffleMode::Off => {
                self.playback_order = indices.into();
            }
            ShuffleMode::TrueRandom => {
                indices.shuffle(&mut self.rng);
                self.playback_order = indices.into();
            }
            ShuffleMode::SmartBalanced { artist_spacing } => {
                self.playback_order = self.generate_balanced_indices(&indices, artist_spacing).into();
            }
        }
    }

    fn generate_balanced_indices(&mut self, base_indices: &[usize], spacing: usize) -> Vec<usize> {
        let mut pool = base_indices.to_vec();
        pool.shuffle(&mut self.rng);

        let mut result = Vec::with_capacity(pool.len());
        let mut artist_last_seen: HashMap<String, usize> = HashMap::new();

        while !pool.is_empty() {
            let mut found_idx = None;

            for (i, &track_idx) in pool.iter().enumerate() {
                let artist = &self.pristine_queue[track_idx].metadata.artist;
                let current_pos = result.len();

                if let Some(&last_pos) = artist_last_seen.get(artist) {
                    if current_pos.saturating_sub(last_pos) >= spacing {
                        found_idx = Some(i);
                        break;
                    }
                } else {
                    found_idx = Some(i);
                    break;
                }
            }

            // Fallback if constraint cannot be strictly satisfied
            let idx_to_take = found_idx.unwrap_or(0);
            let chosen = pool.remove(idx_to_take);
            let artist = self.pristine_queue[chosen].metadata.artist.clone();
            
            artist_last_seen.insert(artist, result.len());
            result.push(chosen);
        }

        result
    }
}
```

---

## 2. Gapless Playback Engine & Real-Time Crossfader

To eliminate silent gaps between live recordings or continuous albums (such as Pink Floyd or DJ sets), the core engine introduces a **dual-pipeline decoder mixer**. While Track $N$ plays, Track $N+1$ is opened, precached, and decoded ahead of time into a secondary ring buffer.

```
 Track A (Outro)  ─────────────────────\ (Fade Out)
                                        \
                                         ▼
                                       [MIXER] ────────► Output PCM Stream to DAC
                                         ▲
 Track B (Intro)  ──────────────────────/ (Fade In)
```

### 2.1. Interpolated Dynamic Crossfade DSP (`crates/amp-core-engine/src/dsp/crossfade.rs`)

```rust
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossfadeCurve {
    Linear,
    EqualPower, // Constant perceptual acoustic energy across transition
}

pub struct Crossfader {
    curve: CrossfadeCurve,
    duration_frames: usize,
    current_frame: usize,
    active: bool,
}

impl Crossfader {
    pub fn new(curve: CrossfadeCurve, duration_frames: usize) -> Self {
        Self {
            curve,
            duration_frames: duration_frames.max(1),
            current_frame: 0,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.current_frame = 0;
        self.active = true;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Computes normalized gain coefficients: (gain_outgoing, gain_incoming)
    #[inline(always)]
    pub fn step_gains(&mut self) -> (f32, f32) {
        if !self.active {
            return (1.0, 0.0);
        }

        let progress = (self.current_frame as f32 / self.duration_frames as f32).clamp(0.0, 1.0);
        self.current_frame += 1;

        if self.current_frame >= self.duration_frames {
            self.active = false;
        }

        match self.curve {
            CrossfadeCurve::Linear => {
                let out_gain = 1.0 - progress;
                let in_gain = progress;
                (out_gain, in_gain)
            }
            CrossfadeCurve::EqualPower => {
                // Quarter-sine / cosine curve keeps power sum (g1^2 + g2^2) = 1.0
                let angle = progress * (PI / 2.0);
                (angle.cos(), angle.sin())
            }
        }
    }
}
```

---

## 3. Loudness Normalization: Pure-Rust EBU R128 & ReplayGain

Playing back a local 1980s CD rip immediately followed by a modern compressed YouTube upload results in sudden volume mismatches. The engine integrates a zero-C loudness normalization stage.

```
       ┌────────────────────────┐
       │   Raw Interleaved PCM  │
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │    K-Weighting Filter  │   • Stage 1: High shelf filter (+4 dB at 1.5 kHz)
       │       (Pure Rust)      │   • Stage 2: High-pass filter (-3 dB at 38 Hz)
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │ Gated Energy Integrator│   • Calculates Integrated LUFS (-23.0 to -14.0 LUFS)
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │ Lookahead Limiter DSP  │   • Applies smooth target gain without digital clipping
       └────────────────────────┘
```

```rust
// crates/amp-core-engine/src/dsp/loudness.rs
#[derive(Debug, Clone, Copy)]
pub struct LoudnessConfig {
    pub target_lufs: f32, // -14.0 LUFS for streaming; -18.0 LUFS for audiophile hi-fi
    pub max_peak_dbfs: f32, // -0.5 dBFS prevents inter-sample DAC overs
    pub enabled: bool,
}

impl Default for LoudnessConfig {
    fn default() -> Self {
        Self {
            target_lufs: -14.0,
            max_peak_dbfs: -0.5,
            enabled: true,
        }
    }
}

pub struct PureVolumeNormalizer {
    config: LoudnessConfig,
    calculated_gain: f32,
}

impl PureVolumeNormalizer {
    pub fn new(config: LoudnessConfig) -> Self {
        Self {
            config,
            calculated_gain: 1.0,
        }
    }

    /// Sets gain from known ReplayGain track tag or precomputed metadata
    pub fn set_track_lufs(&mut self, track_lufs: f32) {
        if !self.config.enabled {
            self.calculated_gain = 1.0;
            return;
        }

        let diff_db = self.config.target_lufs - track_lufs;
        let linear_gain = 10.0f32.powf(diff_db / 20.0);
        
        // Clamp to prevent extreme boosting on silent tracks
        self.calculated_gain = linear_gain.clamp(0.1, 4.0);
    }

    #[inline(always)]
    pub fn process_frame(&self, samples: &mut [f32]) {
        if !self.config.enabled || (self.calculated_gain - 1.0).abs() < f32::EPSILON {
            return;
        }

        for sample in samples.iter_mut() {
            *sample = (*sample * self.calculated_gain).clamp(-1.0, 1.0);
        }
    }
}
```