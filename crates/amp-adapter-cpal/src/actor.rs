//! Dedicated background actor thread managing CPAL stream lifecycle.

#![forbid(unsafe_code)]

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use rtrb::Consumer;
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

pub(crate) enum SinkControlMsg {
    Configure {
        sample_rate: u32,
        channels: u16,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Start {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Pause {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Terminate,
}

pub(crate) fn run_cpal_actor(
    device: Device,
    config: StreamConfig,
    mut consumer: Consumer<f32>,
    cmd_rx: mpsc::Receiver<SinkControlMsg>,
    current_sr: Arc<AtomicU32>,
    current_ch: Arc<AtomicU16>,
) {
    let err_fn = |err| eprintln!("Audio stream callback error: {err}");

    let stream_res = device.build_output_stream(
        &config,
        move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for sample in output.iter_mut() {
                *sample = consumer.pop().unwrap_or(0.0);
            }
        },
        err_fn,
        None,
    );

    let stream = match stream_res {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize audio output stream: {e}");
            while let Ok(msg) = cmd_rx.recv() {
                match msg {
                    SinkControlMsg::Configure { reply, .. } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Start { reply } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Pause { reply } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Stop { reply } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Terminate => break,
                }
            }
            return;
        }
    };

    // The stream is built in paused state. It is controlled via play() and pause()
    while let Ok(msg) = cmd_rx.recv() {
        match msg {
            SinkControlMsg::Configure { sample_rate, channels, reply } => {
                current_sr.store(sample_rate, Ordering::Relaxed);
                current_ch.store(channels, Ordering::Relaxed);
                let _ = reply.send(Ok(()));
            }
            SinkControlMsg::Start { reply } => {
                let res = stream.play().map_err(|e| format!("Stream play failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Pause { reply } => {
                let res = stream.pause().map_err(|e| format!("Stream pause failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Stop { reply } => {
                let res = stream.pause().map_err(|e| format!("Stream stop failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Terminate => break,
        }
    }
}
