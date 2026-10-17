//! Real-time DJ deck playback tick and crossfader processing.
//! Decodes concurrent Deck A and Deck B streams, drives isolator EQs, and updates BPM.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::audio_pusher::update_visualizer_and_push;
use crate::track_loader::load_track_decoder;
use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::dj::DualDeckMixer;
use amp_core_engine::PlaybackDspChain;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn process_dj_tick(
    driver: &AudioPlaybackDriver,
    dsp_chain: &mut PlaybackDspChain,
    dual_mixer: &mut DualDeckMixer,
    decoder_a: &mut Option<AudioStreamDecoder>,
    decoder_b: &mut Option<AudioStreamDecoder>,
    pcm_a: &mut [f32],
    pcm_b: &mut [f32],
    pcm_mixed: &mut [f32],
) {
    let mut dj = driver.dj_audio.lock().unwrap().clone();
    let resolvers = driver.resolvers.lock().unwrap().clone();

    // 1. Deck A track load / reload
    if dj.deck_a_change {
        *decoder_a = None;
        if let Some(ref track) = dj.deck_a_track {
            if let Ok(dec) = load_track_decoder(track, &resolvers) {
                *decoder_a = Some(dec);
            }
        }
        driver.dj_audio.lock().unwrap().deck_a_change = false;
        dj.deck_a_change = false;
    }

    // 2. Deck B track load / reload
    if dj.deck_b_change {
        *decoder_b = None;
        if let Some(ref track) = dj.deck_b_track {
            if let Ok(dec) = load_track_decoder(track, &resolvers) {
                *decoder_b = Some(dec);
            }
        }
        driver.dj_audio.lock().unwrap().deck_b_change = false;
        dj.deck_b_change = false;
    }

    if !dj.deck_a_playing && !dj.deck_b_playing {
        std::thread::sleep(Duration::from_millis(15));
        return;
    }

    // 3. Decode Deck A
    let mut samples_a = 0;
    if dj.deck_a_playing {
        if let Some(ref mut dec) = decoder_a {
            if let Ok(n) = dec.read_interleaved_samples(pcm_a) {
                samples_a = n;
            }
        }
    }

    // 4. Decode Deck B
    let mut samples_b = 0;
    if dj.deck_b_playing {
        if let Some(ref mut dec) = decoder_b {
            if let Ok(n) = dec.read_interleaved_samples(pcm_b) {
                samples_b = n;
            }
        }
    }

    if samples_a == 0 && samples_b == 0 {
        std::thread::sleep(Duration::from_millis(10));
        return;
    }

    // 5. Sync DJ mixer settings
    dual_mixer.deck_a.set_playing(dj.deck_a_playing);
    dual_mixer.deck_b.set_playing(dj.deck_b_playing);
    dual_mixer.deck_a.set_pitch_percent(dj.deck_a_pitch);
    dual_mixer.deck_b.set_pitch_percent(dj.deck_b_pitch);
    dual_mixer.deck_a.set_volume_fader(dj.deck_a_volume);
    dual_mixer.deck_b.set_volume_fader(dj.deck_b_volume);
    dual_mixer.deck_a.isolator_eq.kill_low = dj.deck_a_kill_low;
    dual_mixer.deck_a.isolator_eq.kill_mid = dj.deck_a_kill_mid;
    dual_mixer.deck_a.isolator_eq.kill_high = dj.deck_a_kill_high;
    dual_mixer.deck_b.isolator_eq.kill_low = dj.deck_b_kill_low;
    dual_mixer.deck_b.isolator_eq.kill_mid = dj.deck_b_kill_mid;
    dual_mixer.deck_b.isolator_eq.kill_high = dj.deck_b_kill_high;
    dual_mixer.set_crossfader(dj.crossfader_pos, dj.curve_profile);
    dual_mixer.key_lock_a = dj.key_lock;
    dual_mixer.key_lock_b = dj.key_lock;

    let target_len = pcm_mixed.len().min(samples_a.max(samples_b));
    let mixed_samples = dual_mixer.process_and_mix(
        &pcm_a[..samples_a],
        &pcm_b[..samples_b],
        &mut pcm_mixed[..target_len],
    );

    // Update real-time BPM estimates back to GUI
    {
        let mut shared = driver.dj_audio.lock().unwrap();
        shared.deck_a_bpm = dual_mixer.deck_a_bpm.estimated_bpm();
        shared.deck_b_bpm = dual_mixer.deck_b_bpm.estimated_bpm();
    }

    let eq_on = driver.eq_enabled.load(Ordering::Relaxed);
    let preamp = *driver.eq_preamp.lock().unwrap();
    let gains = *driver.eq_gains.lock().unwrap();
    let vol = *driver.volume.lock().unwrap();

    let (processed, bands) = dsp_chain.process_interleaved(
        &pcm_mixed[..mixed_samples],
        vol,
        preamp,
        &gains,
        eq_on,
    );

    update_visualizer_and_push(driver, &processed, bands);
}
