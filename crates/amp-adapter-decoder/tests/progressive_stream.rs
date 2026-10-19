//! Streams that arrive slower than real time must still decode every sample:
//! a buffer underrun (`WouldBlock`) is a pause, never data loss or end of track.

#![forbid(unsafe_code)]

use amp_adapter_decoder::{AudioStreamDecoder, GenericMediaSource};
use amp_ports::SharedPrecacheBuffer;
use std::io::Read;
use std::time::{Duration, Instant};

fn wav_tone(frames: usize) -> Vec<u8> {
    let mut wav = Vec::new();
    let data_len = (frames * 4) as u32;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&2u16.to_le_bytes()); // stereo
    wav.extend_from_slice(&44_100u32.to_le_bytes());
    wav.extend_from_slice(&(44_100u32 * 4).to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        let s = ((i as f32 * 0.0627).sin() * 16_000.0) as i16;
        wav.extend_from_slice(&s.to_le_bytes());
        wav.extend_from_slice(&s.to_le_bytes());
    }
    wav
}

#[test]
fn decodes_every_sample_when_data_trickles_in() {
    let frames = 22_050; // 0.5 s
    let wav = wav_tone(frames);
    let buffer = SharedPrecacheBuffer::new(wav.len());
    buffer.push_chunk(&wav[..256]);

    // Producer is slower than the decoder: 256 bytes per 2 ms, so nearly every packet
    // read hits an underrun part-way through.
    let writer = buffer.clone();
    let rest = wav[256..].to_vec();
    let producer = std::thread::spawn(move || {
        for chunk in rest.chunks(256) {
            std::thread::sleep(Duration::from_millis(2));
            writer.push_chunk(chunk);
        }
        writer.mark_finished();
    });

    let mut decoder = AudioStreamDecoder::open_stream(buffer).expect("open stream");
    let mut out = vec![0.0f32; 1024];
    let mut decoded = Vec::new();
    loop {
        match decoder.read_interleaved_samples(&mut out) {
            Ok(0) => break,
            Ok(n) => decoded.extend_from_slice(&out[..n]),
            Err(e) if e == "WouldBlock" => std::thread::sleep(Duration::from_millis(1)),
            Err(e) => panic!("underrun must not be fatal: {e}"),
        }
    }
    producer.join().unwrap();

    assert_eq!(decoded.len(), frames * 2, "samples lost across underruns");
    let peak = decoded.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak > 0.4, "decoded tone is silent (peak {peak})");
}

#[test]
fn gives_up_after_the_stall_timeout() {
    let buffer = SharedPrecacheBuffer::new(64);
    let mut source = GenericMediaSource::new(buffer).with_stall_timeout(Duration::from_millis(50));
    let started = Instant::now();
    let err = source.read(&mut [0u8; 16]).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    assert!(started.elapsed() >= Duration::from_millis(50));
    assert!(started.elapsed() < Duration::from_secs(2));
}
