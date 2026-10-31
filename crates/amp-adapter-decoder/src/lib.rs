//! Pure-Rust multi-format audio decoder engine using Symphonia.
//! Decodes MP3, FLAC, WAV, AAC, and Ogg Vorbis into interleaved normalized f32 PCM samples.

#![forbid(unsafe_code)]

pub mod dsd;
pub mod engine;
pub mod opus;
pub mod probe;
pub mod source;
pub mod stream;
pub mod wavpack;

pub use engine::{DecodedStreamSpec, PureDecoderEngine};
pub use source::GenericMediaSource;
pub use stream::{AudioStreamDecoder, SymphoniaDecoderFactory};

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;
    use std::io::Cursor;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;

    /// Helper to generate a minimal valid 16-bit PCM WAV in memory
    fn generate_test_wav_bytes(num_samples: usize, sample_rate: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let data_size = (num_samples * 2 * 2) as u32; // 2 channels, 16-bit (2 bytes per sample)
        let file_size = 36 + data_size;

        // RIFF header
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        // fmt chunk
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        bytes.extend_from_slice(&2u16.to_le_bytes()); // Stereo 2 channels
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * 2 * 2;
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        // data chunk
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());

        // Fill with synthetic sine wave PCM
        for i in 0..num_samples {
            let val = ((i as f32 * 0.1).sin() * 32767.0) as i16;
            bytes.extend_from_slice(&val.to_le_bytes()); // Left
            bytes.extend_from_slice(&val.to_le_bytes()); // Right
        }

        bytes
    }

    #[test]
    fn test_decode_wav_to_ring_buffer() {
        let wav_data = generate_test_wav_bytes(256, 44100);
        let cursor = Cursor::new(wav_data);

        let (mut producer, mut consumer) = RingBuffer::<f32>::new(4096);
        let stop_signal = Arc::new(AtomicBool::new(false));

        let spec =
            PureDecoderEngine::decode_stream_to_ring_buffer(cursor, &mut producer, stop_signal)
                .expect("decoding test wav failed");

        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.channels, 2);

        // Verify samples arrived in the consumer
        let mut sample_count = 0;
        while let Ok(sample) = consumer.pop() {
            assert!((-1.0..=1.0).contains(&sample));
            sample_count += 1;
        }
        assert_eq!(sample_count, 256 * 2); // 256 frames * 2 channels
    }

    #[test]
    fn test_audio_stream_decoder_read_and_seek() {
        let wav_data = generate_test_wav_bytes(512, 44100);
        let cursor = Cursor::new(wav_data);

        let mut decoder = AudioStreamDecoder::open_media_source(cursor)
            .expect("Failed to open audio stream decoder on wav");

        assert_eq!(decoder.sample_rate(), 44100);

        let mut buffer = vec![0.0f32; 256]; // 128 frames
        let read = decoder
            .read_interleaved_samples(&mut buffer)
            .expect("read failed");
        assert_eq!(read, 256);
        assert!((decoder.elapsed().as_secs_f64() - 128.0 / 44100.0).abs() < 1e-5);

        // Test seek
        decoder.seek(Duration::from_millis(5)).expect("seek failed");
        assert!(decoder.elapsed() <= Duration::from_millis(10));
    }

    #[test]
    fn test_audio_stream_decoder_open_stream() {
        let wav_data = generate_test_wav_bytes(512, 44100);
        let cursor = Cursor::new(wav_data);

        // Test opening arbitrary Read + Seek + Send + Sync stream
        let mut decoder = AudioStreamDecoder::open_stream(cursor)
            .expect("Failed to open audio stream decoder via open_stream");

        assert_eq!(decoder.sample_rate(), 44100);
        let mut buffer = vec![0.0f32; 128];
        let read = decoder
            .read_interleaved_samples(&mut buffer)
            .expect("read failed");
        assert_eq!(read, 128);
    }
}
