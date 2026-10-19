//! Real-time audio playback worker thread.
//! Manages Symphonia audio decoders, the DSP chain, gapless/crossfade playback, DJ mode (via
//! `dj_host`) and CPAL ring-buffer backpressure.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::audio_pusher::update_visualizer_and_push;
use crate::dj_host::DjHost;
use crate::track_loader::load_track_decoder;
use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::PlaybackDspChain;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub fn run_playback_loop(driver: AudioPlaybackDriver, sample_rate: u32) {
    let chunk_frames = 512;
    let mut dsp_chain = PlaybackDspChain::new(sample_rate, chunk_frames);
    let mut dj = DjHost::new(sample_rate);

    let mut pcm_single = vec![0.0f32; chunk_frames * 2];
    let mut pcm_aux = vec![0.0f32; chunk_frames * 2];
    let mut pcm_blend = vec![0.0f32; chunk_frames * 2];

    let mut single_decoder: Option<AudioStreamDecoder> = None;
    let mut next_decoder: Option<AudioStreamDecoder> = None;
    let mut crossfader: Option<amp_core_engine::dsp::Crossfader> = None;

    while !std::thread::panicking() {
        dj.accept(&driver);
        if dj.is_active() {
            if let Some(dec) = dj.tick(&driver, &mut dsp_chain, &mut single_decoder) {
                single_decoder = Some(dec);
                next_decoder = None;
                crossfader = None;
            }
            continue;
        }
        process_standard_tick(
            &driver,
            &mut dsp_chain,
            &mut single_decoder,
            &mut next_decoder,
            &mut crossfader,
            &mut pcm_single,
            &mut pcm_aux,
            &mut pcm_blend,
            sample_rate,
        );
        if dj.try_enter(&driver, &mut single_decoder, &dsp_chain, sample_rate) {
            next_decoder = None;
            crossfader = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_standard_tick(
    driver: &AudioPlaybackDriver,
    dsp_chain: &mut PlaybackDspChain,
    decoder: &mut Option<AudioStreamDecoder>,
    next_decoder: &mut Option<AudioStreamDecoder>,
    crossfader: &mut Option<amp_core_engine::dsp::Crossfader>,
    pcm_buffer: &mut [f32],
    pcm_aux: &mut [f32],
    pcm_blend: &mut [f32],
    sample_rate: u32,
) {
    // 1. Handle explicit track change request
    if driver.track_change_signal.swap(false, Ordering::SeqCst) {
        let track_opt = driver.current_track.lock().unwrap().clone();
        *decoder = None;
        *next_decoder = None;
        *crossfader = None;

        if let Some(track) = track_opt {
            let current_resolvers = driver.resolvers.lock().unwrap().clone();
            match load_track_decoder(&track, &current_resolvers) {
                Ok(dec) => {
                    let probed_dur = dec.duration().or(track.metadata.duration);
                    *driver.total_duration.lock().unwrap() = probed_dur;
                    if let Some(cur) = driver.current_track.lock().unwrap().as_mut() {
                        if cur.metadata.duration.is_none() {
                            cur.metadata.duration = probed_dur;
                        }
                    }
                    dsp_chain.set_source_sample_rate(dec.sample_rate());
                    *decoder = Some(dec);
                }
                Err(e) => {
                    eprintln!("Failed to load audio track: {e}");
                    driver.track_finished.store(true, Ordering::SeqCst);
                    driver.is_playing.store(false, Ordering::SeqCst);
                }
            }
        }
    }

    // 2. Handle pause / stopped state
    if !driver.is_playing.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(15));
        return;
    }

    // 3. Handle seek request
    let seek_opt = driver.seek_requested.lock().unwrap().take();
    if let Some(target) = seek_opt {
        if let Some(ref mut dec) = decoder {
            let _ = dec.seek(target);
            *driver.elapsed.lock().unwrap() = dec.elapsed();
            *crossfader = None;
        }
    }

    // 4. Lookahead preload upcoming track if within prefetch threshold
    let xf_secs = *driver.crossfade_seconds.lock().unwrap();
    if decoder.is_some() && next_decoder.is_none() {
        let elapsed = *driver.elapsed.lock().unwrap();
        let total = *driver.total_duration.lock().unwrap();
        let threshold = Duration::from_secs_f32(xf_secs.max(3.0) + 1.0);
        if total.is_some() && elapsed.saturating_add(threshold) >= total.unwrap() {
            let next_t_opt = driver.next_track.lock().unwrap().clone();
            if let Some(ref next_t) = next_t_opt {
                let current_resolvers = driver.resolvers.lock().unwrap().clone();
                if let Ok(pre_dec) = load_track_decoder(next_t, &current_resolvers) {
                    *next_decoder = Some(pre_dec);
                }
            }
        }
    }

    // 5. Decode audio frames (with automix crossfade or gapless stitching)
    let mut read_samples = 0;
    if xf_secs > 0.0 && next_decoder.is_some() {
        let elapsed = *driver.elapsed.lock().unwrap();
        let total = *driver.total_duration.lock().unwrap();
        let in_xf_zone = total.is_some()
            && elapsed.saturating_add(Duration::from_secs_f32(xf_secs)) >= total.unwrap();

        if in_xf_zone && crossfader.is_none() {
            let xf_frames = ((xf_secs * sample_rate as f32) as usize).max(64);
            let mut xf = amp_core_engine::dsp::Crossfader::new(
                amp_core_engine::dsp::CrossfadeCurve::EqualPower,
                xf_frames,
            );
            xf.trigger();
            *crossfader = Some(xf);
        }
    }

    if let Some(ref mut xf) = crossfader {
        if xf.is_active() && decoder.is_some() && next_decoder.is_some() {
            let n_out = decoder
                .as_mut()
                .unwrap()
                .read_interleaved_samples(pcm_buffer)
                .unwrap_or(0);
            let n_in = next_decoder
                .as_mut()
                .unwrap()
                .read_interleaved_samples(pcm_aux)
                .unwrap_or(0);

            if n_out == 0 {
                handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                *crossfader = None;
                pcm_buffer[..n_in].copy_from_slice(&pcm_aux[..n_in]);
                read_samples = n_in;
            } else {
                let frames = (n_out.min(n_in).min(pcm_blend.len())) / 2;
                xf.process_interleaved_blend(
                    &pcm_buffer[..frames * 2],
                    &pcm_aux[..frames * 2],
                    &mut pcm_blend[..frames * 2],
                );
                pcm_buffer[..frames * 2].copy_from_slice(&pcm_blend[..frames * 2]);
                read_samples = frames * 2;
                *driver.elapsed.lock().unwrap() = decoder.as_ref().unwrap().elapsed();
                if !xf.is_active() {
                    handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                    *crossfader = None;
                }
            }
        } else {
            handoff_to_next(driver, decoder, next_decoder, dsp_chain);
            *crossfader = None;
        }
    } else if let Some(ref mut dec) = decoder {
        match dec.read_interleaved_samples(pcm_buffer) {
            Ok(n) if n > 0 => {
                read_samples = n;
                *driver.elapsed.lock().unwrap() = dec.elapsed();
                // If partial chunk at EOF, stitch from next_decoder if available
                if n < pcm_buffer.len() && next_decoder.is_some() {
                    let remainder = &mut pcm_buffer[n..];
                    if let Ok(next_n) = next_decoder
                        .as_mut()
                        .unwrap()
                        .read_interleaved_samples(remainder)
                    {
                        read_samples += next_n;
                    }
                    handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                }
            }
            Err(ref e) if e.contains("WouldBlock") => {
                std::thread::sleep(Duration::from_millis(10));
                return;
            }
            Ok(_) | Err(_) => {
                if next_decoder.is_some() {
                    handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                    if let Some(ref mut new_dec) = decoder {
                        if let Ok(n) = new_dec.read_interleaved_samples(pcm_buffer) {
                            read_samples = n;
                        }
                    }
                } else {
                    driver.track_finished.store(true, Ordering::SeqCst);
                    driver.is_playing.store(false, Ordering::SeqCst);
                    *decoder = None;
                }
            }
        }
    } else {
        std::thread::sleep(Duration::from_millis(15));
        return;
    }

    if read_samples == 0 {
        return;
    }

    // 6. DSP & push to sink (with track LUFS loudness normalization)
    let eq_on = driver.eq_enabled.load(Ordering::Relaxed);
    let preamp = *driver.eq_preamp.lock().unwrap();
    let gains = *driver.eq_gains.lock().unwrap();
    let vol = *driver.volume.lock().unwrap();
    let track_lufs = driver
        .current_track
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|t| t.metadata.integrated_lufs());

    let (processed, bands) = dsp_chain.process_interleaved_with_lufs(
        &pcm_buffer[..read_samples],
        vol,
        preamp,
        &gains,
        eq_on,
        track_lufs,
    );

    update_visualizer_and_push(driver, &processed, bands);
}

fn handoff_to_next(
    driver: &AudioPlaybackDriver,
    decoder: &mut Option<AudioStreamDecoder>,
    next_decoder: &mut Option<AudioStreamDecoder>,
    dsp_chain: &mut PlaybackDspChain,
) {
    *decoder = next_decoder.take();
    let next_t = driver.next_track.lock().unwrap().take();
    *driver.current_track.lock().unwrap() = next_t;
    *driver.total_duration.lock().unwrap() = decoder.as_ref().and_then(|d| d.duration());
    *driver.elapsed.lock().unwrap() = decoder
        .as_ref()
        .map(|d| d.elapsed())
        .unwrap_or(Duration::ZERO);
    if let Some(ref d) = decoder {
        dsp_chain.set_source_sample_rate(d.sample_rate());
    }
    driver.track_advanced_signal.store(true, Ordering::SeqCst);
}
