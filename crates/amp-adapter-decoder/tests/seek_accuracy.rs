//! Seeking is sample-accurate: after a seek, the first decoded frame is exactly the requested
//! one and `elapsed` reports it, for any target (not just packet boundaries).

use amp_adapter_decoder::AudioStreamDecoder;
use std::io::Cursor;
use std::time::Duration;

const RATE: u32 = 44_100;

/// A stereo WAV whose sample values count the frame number (mod 32768), so a decoded value
/// tells which frame it came from.
fn counting_wav(frames: u32) -> Vec<u8> {
    let data_len = frames * 4;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&RATE.to_le_bytes());
    w.extend_from_slice(&(RATE * 4).to_le_bytes());
    w.extend_from_slice(&4u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        let v = (i % 32_768) as i16;
        w.extend_from_slice(&v.to_le_bytes());
        w.extend_from_slice(&v.to_le_bytes());
    }
    w
}

fn frame_of(sample: f32) -> u32 {
    (sample * 32_768.0).round() as u32
}

#[test]
fn seek_lands_on_the_exact_frame() {
    let wav = counting_wav(RATE * 4);
    for target in [1_234u32, 20_000, 44_100, 77_777, 100_003] {
        let mut d = AudioStreamDecoder::open_stream(Cursor::new(wav.clone())).expect("open");
        d.seek(Duration::from_secs_f64(f64::from(target) / f64::from(RATE)))
            .expect("seek");
        let mut out = vec![0.0f32; 64];
        let n = d.read_interleaved_samples(&mut out).expect("read");
        assert!(n >= 2);
        assert_eq!(frame_of(out[0]), target % 32_768, "seek to frame {target}");
        assert_eq!(frame_of(out[2]), (target + 1) % 32_768);
        let elapsed = d.elapsed().as_secs_f64() * f64::from(RATE);
        assert!(
            (elapsed - f64::from(target + 32)).abs() < 1.0,
            "elapsed {elapsed}"
        );
    }
}
