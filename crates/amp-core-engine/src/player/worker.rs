//! Event-driven real-time playback worker thread (`TASK-125`, `TASK-127`, `TASK-128`, `TASK-129`).
//!
//! Parks on a `Condvar` when idle/paused, retains unpushed PCM in `pending_out` across pause
//! and device switches, flushes stale ring-buffer epochs via `OutputClock`, and drives
//! speaker-aligned position and visualizer updates.

#![forbid(unsafe_code)]

use super::dj_host::DjHost;
use super::loader::{spawn_track_load, PendingLoad, DEFAULT_PRELOAD_MAX_RETRIES};
use super::pusher::{
    drain_pending_to_ring, update_clock_position_and_tap, update_visualizer_fallback,
    watch_stream_buffer,
};
use super::PlayerEngine;
use crate::dsp::{CrossfadeCurve, Crossfader, SpectrumVisualizer};
use crate::events::{PlayerErrorKind, PlayerEvent};
use crate::PlaybackDspChain;
use amp_domain::{AudioSource, PlaybackState, Track};
use amp_ports::{DecodeError, PcmStreamPort};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn run_playback_loop(driver: PlayerEngine, sample_rate: u32) {
    let chunk_frames = 512;
    let mut dsp_chain = PlaybackDspChain::new(sample_rate, chunk_frames);
    let mut dj = DjHost::new(sample_rate);
    let mut tap_visualizer = SpectrumVisualizer::new();
    let mut tap_buf = vec![0.0f32; 2048];
    let mut pcm_single = vec![0.0f32; chunk_frames * 2];
    let mut pcm_aux = vec![0.0f32; chunk_frames * 2];
    let mut pcm_blend = vec![0.0f32; chunk_frames * 2];
    let mut pending_out: VecDeque<f32> = VecDeque::with_capacity(chunk_frames * 4);
    let mut single_decoder: Option<Box<dyn PcmStreamPort>> = None;
    let mut next_decoder: Option<Box<dyn PcmStreamPort>> = None;
    let mut pending_primary: Option<PendingLoad> = None;
    let mut pending_preload: Option<PendingLoad> = None;
    let mut preload_failed_track_id = None;
    let mut crossfader: Option<Crossfader> = None;

    while !std::thread::panicking() && !driver.is_shutdown() {
        driver.drain_commands();
        dj.accept(&driver);
        if dj.is_active() {
            if let Some(dec) = dj.tick(&driver, &mut dsp_chain, &mut single_decoder) {
                single_decoder = Some(dec);
                next_decoder = None;
                crossfader = None;
                pending_out.clear();
            }
            continue;
        }
        process_standard_tick(
            &driver,
            &mut dsp_chain,
            &mut tap_visualizer,
            &mut tap_buf,
            &mut single_decoder,
            &mut next_decoder,
            &mut pending_primary,
            &mut pending_preload,
            &mut preload_failed_track_id,
            &mut crossfader,
            &mut pcm_single,
            &mut pcm_aux,
            &mut pcm_blend,
            &mut pending_out,
            sample_rate,
        );
        if pending_out.is_empty()
            && dj.try_enter(&driver, &mut single_decoder, &dsp_chain, sample_rate)
        {
            next_decoder = None;
            crossfader = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_standard_tick(
    driver: &PlayerEngine,
    dsp_chain: &mut PlaybackDspChain,
    tap_visualizer: &mut SpectrumVisualizer,
    tap_buf: &mut [f32],
    decoder: &mut Option<Box<dyn PcmStreamPort>>,
    next_decoder: &mut Option<Box<dyn PcmStreamPort>>,
    pending_primary: &mut Option<PendingLoad>,
    pending_preload: &mut Option<PendingLoad>,
    preload_failed_track_id: &mut Option<amp_domain::TrackId>,
    crossfader: &mut Option<Crossfader>,
    pcm_buffer: &mut [f32],
    pcm_aux: &mut [f32],
    pcm_blend: &mut [f32],
    pending_out: &mut VecDeque<f32>,
    sample_rate: u32,
) {
    // 1. Handle explicit track change request
    if driver.track_change_signal.swap(false, Ordering::SeqCst) {
        if let Some(mut p) = pending_primary.take() {
            p.cancel();
        }
        if let Some(mut p) = pending_preload.take() {
            p.cancel();
        }
        *preload_failed_track_id = None;
        *decoder = None;
        *next_decoder = None;
        *crossfader = None;
        pending_out.clear();
        driver.clock.flush_and_reset(Duration::ZERO);

        let track_opt = driver.current_track.lock().unwrap().clone();
        if let Some(track) = track_opt {
            match &track.source {
                AudioSource::LocalFile { path } => match driver
                    .decoder_factory
                    .open_file(path)
                    .and_then(|dec| super::segment::apply_segment(&track, dec))
                {
                    Ok(dec) => activate_loaded_decoder(driver, dsp_chain, decoder, track, dec),
                    Err(e) => handle_load_failure(driver, &track, &e),
                },
                _ => {
                    driver.set_state_and_emit(PlaybackState::Loading);
                    let resolvers = driver.resolvers.lock().unwrap().clone();
                    *pending_primary = Some(spawn_track_load(
                        &driver.runtime_handle,
                        track,
                        Arc::clone(&driver.decoder_factory),
                        resolvers,
                        driver.resolve_timeout(),
                        0,
                        driver.event_bus.clone(),
                    ));
                }
            }
        }
    }

    // 1b. Poll non-blocking primary stream resolution
    if let Some(ref mut pending) = pending_primary {
        match pending.try_take() {
            Some(Ok(opened)) => {
                *pending_primary = None;
                activate_loaded_decoder(driver, dsp_chain, decoder, opened.track, opened.decoder);
            }
            Some(Err(e)) => {
                let failed_track = pending.track.clone();
                *pending_primary = None;
                handle_load_failure(driver, &failed_track, &e);
            }
            None => {
                driver.wait_for_wake(Duration::from_millis(5));
                return;
            }
        }
    }

    // 2. Handle seek request (supported while playing or paused)
    let seek_opt = driver.seek_requested.lock().unwrap().take();
    if let Some(target) = seek_opt {
        pending_out.clear();
        *crossfader = None;
        let landed = decoder
            .as_mut()
            .and_then(|dec| dec.seek(target).ok())
            .unwrap_or(target);
        driver.clock.flush_and_reset(landed);
        *driver.elapsed.lock().unwrap() = landed;
        let total = *driver.total_duration.lock().unwrap();
        driver.emit(PlayerEvent::PositionUpdated {
            elapsed: landed,
            total,
        });
    }

    // 3. Park on Condvar when paused or stopped (0% idle CPU)
    if !driver.is_playing.load(Ordering::Acquire) {
        driver.wait_for_wake(Duration::from_millis(50));
        return;
    }

    // 4. Drain any retained `pending_out` frames before decoding new ones
    if !pending_out.is_empty() {
        drain_pending_to_ring(driver, pending_out);
        update_clock_position_and_tap(driver, tap_visualizer, tap_buf);
        if !pending_out.is_empty() {
            // The ring is full (150 ms of audio by default): 10 ms is well inside that.
            // Commands still wake us at once.
            driver.wait_for_wake(Duration::from_millis(10));
            return;
        }
    }

    // 5. Lookahead preload upcoming track if within prefetch threshold
    let xf_secs = *driver.crossfade_seconds.lock().unwrap();
    if decoder.is_some() && next_decoder.is_none() {
        poll_or_start_preload(
            driver,
            pending_preload,
            next_decoder,
            preload_failed_track_id,
            xf_secs,
        );
    }

    // 6. Decode audio frames (with automix crossfade or gapless stitching, TASK-145)
    if xf_secs > 0.0 && next_decoder.is_some() && crossfader.is_none() {
        let same_album = match (
            driver.current_track.lock().ok().and_then(|t| t.clone()),
            driver.peek_or_next_preload_track(),
        ) {
            (Some(cur), Some(nxt)) => {
                let cur_alb = cur.metadata.album.as_deref().unwrap_or("").trim();
                let nxt_alb = nxt.metadata.album.as_deref().unwrap_or("").trim();
                !cur_alb.is_empty() && cur_alb.eq_ignore_ascii_case(nxt_alb)
            }
            _ => false,
        };

        if !same_album {
            let elapsed = *driver.elapsed.lock().unwrap();
            let total = *driver.total_duration.lock().unwrap();
            if total
                .is_some_and(|tot| elapsed.saturating_add(Duration::from_secs_f32(xf_secs)) >= tot)
            {
                let xf_frames = ((xf_secs * sample_rate as f32) as usize).max(64);
                let mut xf = Crossfader::new(CrossfadeCurve::EqualPower, xf_frames);
                xf.trigger();
                *crossfader = Some(xf);
            }
        }
    }

    let read_samples = if let Some(ref mut xf) = crossfader {
        if xf.is_active() && decoder.is_some() && next_decoder.is_some() {
            let n_out = decoder
                .as_mut()
                .unwrap()
                .read_interleaved(pcm_buffer)
                .unwrap_or(0);
            let n_in = next_decoder
                .as_mut()
                .unwrap()
                .read_interleaved(pcm_aux)
                .unwrap_or(0);
            if n_out == 0 {
                handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                *crossfader = None;
                pcm_buffer[..n_in].copy_from_slice(&pcm_aux[..n_in]);
                n_in
            } else {
                let frames = (n_out.min(n_in).min(pcm_blend.len())) / 2;
                xf.process_interleaved_blend(
                    &pcm_buffer[..frames * 2],
                    &pcm_aux[..frames * 2],
                    &mut pcm_blend[..frames * 2],
                );
                pcm_buffer[..frames * 2].copy_from_slice(&pcm_blend[..frames * 2]);
                if !xf.is_active() {
                    handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                    *crossfader = None;
                }
                frames * 2
            }
        } else {
            handoff_to_next(driver, decoder, next_decoder, dsp_chain);
            *crossfader = None;
            0
        }
    } else if let Some(ref mut dec) = decoder {
        match dec.read_interleaved(pcm_buffer) {
            Ok(n) if n > 0 => {
                // Data again after a WouldBlock. While the output is held to refill
                // (`watch_stream_buffer`), that decides when playing resumes.
                if matches!(driver.playback_state(), PlaybackState::Buffering { .. })
                    && !driver.clock.is_paused()
                {
                    driver.set_state_and_emit(PlaybackState::Playing);
                }
                let mut total_n = n;
                if n < pcm_buffer.len() && next_decoder.is_some() {
                    if let Ok(next_n) = next_decoder
                        .as_mut()
                        .unwrap()
                        .read_interleaved(&mut pcm_buffer[n..])
                    {
                        total_n += next_n;
                    }
                    handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                }
                total_n
            }
            Err(DecodeError::WouldBlock) => {
                driver.set_state_and_emit(PlaybackState::Buffering {
                    progress_percent: 50,
                });
                driver.wait_for_wake(Duration::from_millis(8));
                return;
            }
            Ok(_) | Err(_) => {
                if next_decoder.is_some() {
                    handoff_to_next(driver, decoder, next_decoder, dsp_chain);
                    decoder
                        .as_mut()
                        .and_then(|d| d.read_interleaved(pcm_buffer).ok())
                        .unwrap_or(0)
                } else if let Some(next_queued) = driver.take_next_track_on_eof() {
                    *decoder = None;
                    driver.play(next_queued);
                    return;
                } else {
                    driver.track_finished.store(true, Ordering::SeqCst);
                    driver.is_playing.store(false, Ordering::SeqCst);
                    driver.set_state_and_emit(PlaybackState::Stopped);
                    *decoder = None;
                    0
                }
            }
        }
    } else {
        driver.stream_format.clear();
        driver.wait_for_wake(Duration::from_millis(20));
        return;
    };

    if read_samples == 0 {
        return;
    }

    // 7. DSP processing & ring-buffer push
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

    if let Some(dec) = decoder.as_ref() {
        driver
            .stream_format
            .record(dec.sample_rate(), dsp_chain.last_was_bit_perfect());
    }
    update_visualizer_fallback(driver, &processed, bands);
    pending_out.extend(processed.iter().copied());
    drain_pending_to_ring(driver, pending_out);
    watch_stream_buffer(driver);
    update_clock_position_and_tap(driver, tap_visualizer, tap_buf);
}

fn poll_or_start_preload(
    driver: &PlayerEngine,
    pending_preload: &mut Option<PendingLoad>,
    next_decoder: &mut Option<Box<dyn PcmStreamPort>>,
    preload_failed_track_id: &mut Option<amp_domain::TrackId>,
    xf_secs: f32,
) {
    if let Some(ref mut pre) = pending_preload {
        if let Some(res) = pre.try_take() {
            match res {
                Ok(opened) => *next_decoder = Some(opened.decoder),
                Err(_) => *preload_failed_track_id = Some(pre.track.id),
            }
            *pending_preload = None;
        }
        return;
    }
    let elapsed = *driver.elapsed.lock().unwrap();
    let Some(tot) = *driver.total_duration.lock().unwrap() else {
        return;
    };
    let threshold = Duration::from_secs_f32(xf_secs.max(3.0) + 1.0);
    if elapsed.saturating_add(threshold) >= tot {
        if let Some(next_t) = driver.peek_or_next_preload_track() {
            if *preload_failed_track_id != Some(next_t.id) {
                let resolvers = driver.resolvers.lock().unwrap().clone();
                *pending_preload = Some(spawn_track_load(
                    &driver.runtime_handle,
                    next_t,
                    Arc::clone(&driver.decoder_factory),
                    resolvers,
                    driver.resolve_timeout(),
                    DEFAULT_PRELOAD_MAX_RETRIES,
                    driver.event_bus.clone(),
                ));
            }
        }
    }
}

fn activate_loaded_decoder(
    driver: &PlayerEngine,
    dsp_chain: &mut PlaybackDspChain,
    decoder: &mut Option<Box<dyn PcmStreamPort>>,
    mut track: Track,
    dec: Box<dyn PcmStreamPort>,
) {
    let probed_dur = dec.duration().or(track.metadata.duration);
    *driver.total_duration.lock().unwrap() = probed_dur;
    if track.metadata.duration.is_none() {
        track.metadata.duration = probed_dur;
    }
    *driver.current_track.lock().unwrap() = Some(track.clone());
    dsp_chain.set_source_sample_rate(dec.sample_rate());
    driver.clock.flush_and_reset(Duration::ZERO);
    if driver.is_playing.load(Ordering::Acquire) {
        driver.clock.resume();
        driver.set_state_and_emit(PlaybackState::Playing);
    }
    driver.emit(PlayerEvent::TrackStarted(Box::new(track)));
    *decoder = Some(dec);
}

fn handle_load_failure(driver: &PlayerEngine, track: &Track, err: &DecodeError) {
    let msg = err.to_string();
    driver.track_finished.store(true, Ordering::SeqCst);
    driver.is_playing.store(false, Ordering::SeqCst);
    driver.set_state_and_emit(PlaybackState::Error);
    driver.emit(PlayerEvent::Error {
        kind: PlayerErrorKind::Decode,
        source: Some(track.source.clone()),
        message: msg.clone(),
        recoverable: true,
    });
    driver.emit(PlayerEvent::ErrorNotification(msg));
}

fn handoff_to_next(
    driver: &PlayerEngine,
    decoder: &mut Option<Box<dyn PcmStreamPort>>,
    next_decoder: &mut Option<Box<dyn PcmStreamPort>>,
    dsp_chain: &mut PlaybackDspChain,
) {
    *decoder = next_decoder.take();
    let next_t = driver.take_next_track_for_handoff();
    *driver.current_track.lock().unwrap() = next_t.clone();
    *driver.total_duration.lock().unwrap() = decoder.as_ref().and_then(|d| d.duration());
    let dec_elapsed = decoder
        .as_ref()
        .map(|d| d.elapsed())
        .unwrap_or(Duration::ZERO);
    driver.clock.mark_seamless_transition(dec_elapsed);
    *driver.elapsed.lock().unwrap() = dec_elapsed;
    if let Some(ref d) = decoder {
        dsp_chain.set_source_sample_rate(d.sample_rate());
    }
    if let Some(t) = next_t {
        driver.emit(PlayerEvent::TrackStarted(Box::new(t)));
    }
    driver.track_advanced_signal.store(true, Ordering::SeqCst);
}
