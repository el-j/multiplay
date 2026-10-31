//! Live canary integration test for YouTube stream resolution and Symphonia audio decoding.
//!
//! Marked `#[ignore]` so it does not run during standard offline unit tests.
//! Triggered by CI scheduled workflow `.github/workflows/canary.yml` or manual invocation:
//! `cargo test -p amp-adapter-youtube --test canary -- --ignored`

use amp_adapter_decoder::AudioStreamDecoder;
use amp_adapter_youtube::PureYouTubeAdapter;
use amp_domain::AudioSource;
use amp_ports::StreamResolverPort;

#[tokio::test]
#[ignore = "Live network canary test for scheduled CI"]
async fn test_canary_youtube_resolve_and_decode_5s() {
    let video_id = std::env::var("CANARY_VIDEO_ID").unwrap_or_else(|_| "jNQXAC9IVRw".to_string());
    let adapter = PureYouTubeAdapter::new().expect("Failed to initialize YouTube adapter");

    let source = AudioSource::YouTube { video_id };
    let stream = adapter
        .resolve_stream(&source)
        .await
        .expect("Canary: failed to resolve YouTube stream");

    // Open in pure-Rust Symphonia decoder with m4a / audio/mp4 hint
    let mut decoder =
        AudioStreamDecoder::open_stream_with_hint(stream, Some("m4a"), Some("audio/mp4"))
            .expect("Canary: failed to open audio stream in Symphonia decoder");

    let sample_rate = decoder.sample_rate();
    assert!(sample_rate > 0, "Canary: sample rate must be > 0");

    // Decode 5 seconds of stereo audio frames (2 channels)
    let target_samples = (sample_rate as usize) * 2 * 5;
    let mut decoded_samples = 0;
    let mut chunk_buf = vec![0.0f32; 4096];

    while decoded_samples < target_samples {
        let n = decoder
            .read_interleaved_samples(&mut chunk_buf)
            .expect("Canary: failed reading samples from decoder");
        if n == 0 {
            break;
        }
        decoded_samples += n;
    }

    assert!(
        decoded_samples >= target_samples,
        "Canary: decoded only {decoded_samples} samples, expected at least {target_samples} (5s)"
    );
}
