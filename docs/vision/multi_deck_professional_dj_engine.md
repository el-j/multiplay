# Multi-Deck Professional DJ Engine & Universal Source Mixer

This document specifies the multi-deck DJ architecture within the pure-Rust framework. It enables independent cueing, manual pitch and speed shifting, automatic beat detection, loop rolls, and crossfader curve control across **any source** (mixing a local FLAC on Deck A with a YouTube live set on Deck B and a Soulseek MP3 on Deck C).

---

## 1. Multi-Deck Pipeline Architecture

```
 ┌────────────────────────────────────────────────────────────────────────────────────────┐
 │                              MULTI-DECK SYSTEM TOPOLOGY                                │
 └────────────────────────────────────────────────────────────────────────────────────────┘

    DECK A (Source: Local FLAC)               DECK B (Source: YouTube Stream)
    ┌───────────────────────────┐             ┌───────────────────────────┐
    │  Stream Precache Buffer   │             │   Stream Precache Buffer  │
    └─────────────┬─────────────┘             └─────────────┬─────────────┘
                  │                                         │
                  ▼                                         ▼
    ┌───────────────────────────┐             ┌───────────────────────────┐
    │ Symphonia Real-Time Stream│             │ Symphonia Real-Time Stream│
    └─────────────┬─────────────┘             └─────────────┬─────────────┘
                  │                                         │
                  ▼                                         ▼
    ┌───────────────────────────┐             ┌───────────────────────────┐
    │ Rubberband / WSOLA Engine │             │ Rubberband / WSOLA Engine │
    │ (Pitch / Tempo Shifting)  │             │ (Pitch / Tempo Shifting)  │
    └─────────────┬─────────────┘             └─────────────┬─────────────┘
                  │                                         │
                  ▼                                         ▼
    ┌───────────────────────────┐             ┌───────────────────────────┐
    │ 3-Band Isolator Kill EQs  │             │ 3-Band Isolator Kill EQs  │
    │  (Low / Mid / High Dials) │             │  (Low / Mid / High Dials) │
    └─────────────┬─────────────┘             └─────────────┬─────────────┘
                  │                                         │
                  ▼                                         ▼
    ┌───────────────────────────┐             ┌───────────────────────────┐
    │ Cue Bus (Headphone PFL)   │             │ Cue Bus (Headphone PFL)   │
    └─────────────┬─────────────┘             └─────────────┬─────────────┘
                  │                                         │
                  └────────────────────┬────────────────────┘
                                       │
                                       ▼
                       ┌───────────────────────────────┐
                       │ Crossfader / Transform Engine │
                       │    (Linear / Sharp Scratch)   │
                       └───────────────┬───────────────┘
                                       │
                                       ▼
                       ┌───────────────────────────────┐
                       │      Master Stereo Out        │
                       │     (CoreAudio Hardware)      │
                       └───────────────────────────────┘
```

---

## 2. Deck State & Time-Stretching WSOLA Engine

To change tempo without altering pitch (Key Lock / Master Tempo), the engine provides an allocation-free **Waveform Similarity Overlap-Add (WSOLA)** time-stretching kernel written in safe Rust.

### 2.1. Deck Controller Model (`crates/amp-core-engine/src/dj/deck.rs`)

```rust
use amp_domain::Track;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckId {
    DeckA,
    DeckB,
    DeckC,
    DeckD,
}

#[derive(Debug, Clone)]
pub struct DeckChannelState {
    pub id: DeckId,
    pub loaded_track: Option<Track>,
    pub is_playing: bool,
    pub is_cue_headphone_active: bool,
    pub playback_rate: f32, // 1.0 = normal, 0.92 = -8%, 1.08 = +8% pitch slider
    pub bpm: Option<f32>,
    pub beat_grid_offset_ms: u32,
    
    // 3-Band DJ Kill EQ (-inf dB to +6 dB)
    pub eq_low: f32,
    pub eq_mid: f32,
    pub eq_high: f32,
    pub eq_low_kill: bool,
    pub eq_mid_kill: bool,
    pub eq_high_kill: bool,
    
    // Looping Engine
    pub loop_active: bool,
    pub loop_in_pos: Option<Duration>,
    pub loop_out_pos: Option<Duration>,
    pub volume_fader: f32, // 0.0 to 1.0
}

impl DeckChannelState {
    pub fn new(id: DeckId) -> Self {
        Self {
            id,
            loaded_track: None,
            is_playing: false,
            is_cue_headphone_active: false,
            playback_rate: 1.0,
            bpm: None,
            beat_grid_offset_ms: 0,
            eq_low: 1.0,
            eq_mid: 1.0,
            eq_high: 1.0,
            eq_low_kill: false,
            eq_mid_kill: false,
            eq_high_kill: false,
            loop_active: false,
            loop_in_pos: None,
            loop_out_pos: None,
            volume_fader: 1.0,
        }
    }

    pub fn set_tempo_nudge(&mut self, percentage: f32) {
        self.playback_rate = (1.0 + (percentage / 100.0)).clamp(0.5, 2.0);
    }
}
```

---

## 3. Pure-Rust Transient Energy BPM & Beat Detector

To enable visual beat-matching grids in the TUI/GUI and sync tracks from different sources, the engine executes a continuous spectral-flux onset detection algorithm.

```rust
// crates/amp-core-engine/src/dj/beat_detector.rs
const HISTORY_BINS: usize = 43; // ~1 second of audio at 44.1kHz buffer size 1024

pub struct RealtimeBpmDetector {
    energy_history: [f32; HISTORY_BINS],
    history_idx: usize,
    sample_rate: u32,
    samples_since_last_beat: usize,
    estimated_bpm: f32,
}

impl RealtimeBpmDetector {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            energy_history: [0.0; HISTORY_BINS],
            history_idx: 0,
            sample_rate,
            samples_since_last_beat: 0,
            estimated_bpm: 120.0,
        }
    }

    /// Analyzes an interleaved PCM frame and detects energetic transients
    pub fn process_block(&mut self, samples: &[f32]) -> bool {
        let mut frame_energy = 0.0f32;
        for &s in samples {
            frame_energy += s * s;
        }

        // Calculate average historical energy
        let avg_energy: f32 = self.energy_history.iter().sum::<f32>() / HISTORY_BINS as f32;
        let variance: f32 = self.energy_history.iter().map(|e| (e - avg_energy).powi(2)).sum::<f32>() / HISTORY_BINS as f32;
        
        // Dynamic thresholding constant (C) based on variance
        let c = (-0.0025714 * variance) + 1.5142857;
        let is_beat = frame_energy > (c * avg_energy) && self.samples_since_last_beat > (self.sample_rate as usize / 3);

        if is_beat {
            let samples_between_beats = self.samples_since_last_beat;
            self.samples_since_last_beat = 0;

            let instant_bpm = 60.0 / (samples_between_beats as f32 / self.sample_rate as f32);
            if (60.0..=200.0).contains(&instant_bpm) {
                // Low-pass filter the BPM estimator to avoid erratic swings
                self.estimated_bpm = (self.estimated_bpm * 0.9) + (instant_bpm * 0.1);
            }
        } else {
            self.samples_since_last_beat += samples.len();
        }

        self.energy_history[self.history_idx] = frame_energy;
        self.history_idx = (self.history_idx + 1) % HISTORY_BINS;

        is_beat
    }

    pub fn current_bpm(&self) -> f32 {
        self.estimated_bpm
    }
}
```

---

## 4. Hardware-Style DJ Console TUI Interface

```
┌─ [DECK A] (Local FLAC) ─────────────────────┬─ [DECK B] (YouTube Stream) ─────────────────┐
│ Aphex Twin - Windowlicker                   │ Burial - Archangel                          │
│ 128.0 BPM [SYNC]   KEY: 4A    PITCH: +0.0%  │ 134.2 BPM [SYNC]   KEY: 6A    PITCH: -4.6%  │
│ 02:41 / 06:07 ▍▍▍▍▍▍▍▍▍▍░░░░░░░░░░░░░░░░    │ 01:14 / 04:00 ▍▍▍▍░░░░░░░░░░░░░░░░░░░░░░░░  │
│ [BEAT] |   |   |   | [HOT CUE: 1 2 3 4]     │ [BEAT]   |   |   |   [HOT CUE: 1 2 3 4]     │
│ EQ: HI: [■] MID: [■] LOW: [KILL]  VOL: 100% │ EQ: HI: [■] MID: [■] LOW: [■]    VOL:  85%  │
└─────────────────────────────────────────────┴─────────────────────────────────────────────┘
                                  CROSSFADER
                  DECK A [═════════════●═════════════] DECK B
                       [CUT SHARP]  [PFL A]  [PFL B]
```