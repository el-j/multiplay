//! Every decoder keeps the `PcmStreamPort` contract (TASK-187): Symphonia's formats, our Opus,
//! DSD and WavPack decoders, opened from files and from streams.

#![forbid(unsafe_code)]

use amp_adapter_decoder::SymphoniaDecoderFactory;
use amp_ports::{DecoderFactoryPort, PcmStreamPort};
use amp_test_support::check_pcm_stream;
use std::path::Path;

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn decoders_opened_from_files_keep_the_contract() {
    for name in [
        "flac_16_44k.flac",
        "flac_24_96k.flac",
        "flac_5_1_48k.flac",
        "wav_16.wav",
        "wav_f32.wav",
        "mp3_cbr_128.mp3",
        "mp3_vbr.mp3",
        "aac.m4a",
        "alac.m4a",
        "vorbis.ogg",
        "opus_stereo.opus",
        "opus_5_1.opus",
        "aiff_16.aiff",
        "flac.mka",
        "dsd_stereo.dsf",
        "dsd_stereo.dff",
        "wavpack_16.wv",
    ] {
        let path = fixture(name);
        check_pcm_stream(name, &|| -> Box<dyn PcmStreamPort> {
            SymphoniaDecoderFactory.open_file(&path).expect("opens")
        });
    }
}

#[test]
fn decoders_opened_from_streams_keep_the_contract() {
    for name in [
        "flac_16_44k.flac",
        "mp3_cbr_128.mp3",
        "opus_stereo.opus",
        "dsd_stereo.dsf",
        "wavpack_16.wv",
    ] {
        let bytes = std::fs::read(fixture(name)).unwrap();
        check_pcm_stream(
            &format!("{name} (stream)"),
            &|| -> Box<dyn PcmStreamPort> {
                SymphoniaDecoderFactory
                    .open_stream(Box::new(std::io::Cursor::new(bytes.clone())))
                    .expect("opens")
            },
        );
    }
}
