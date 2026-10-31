//! Opus decoding for Symphonia (TASK-229, ADR-005): wraps the pure-Rust `rusty-opus` decoder
//! as a Symphonia audio decoder, so Ogg Opus and Opus in WebM/Matroska play through the normal
//! probe/decode path.
//!
//! Opus always decodes at 48 kHz. Surround streams (mapping family 1) come out in Vorbis channel
//! order and are reordered into Symphonia's channel order; the header's output gain is applied
//! and, for gapless playback, each packet's encoder delay and padding are trimmed.

#![forbid(unsafe_code)]

use rusty_opus::multistream::OpusMSDecoder;
use symphonia::core::audio::{
    AsGenericAudioBufferRef, AudioBuffer, AudioMut, AudioSpec, Channels, GenericAudioBufferRef,
    Position,
};
use symphonia::core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia::core::codecs::audio::{
    AudioCodecParameters, AudioDecoder, AudioDecoderOptions, FinalizeResult,
};
use symphonia::core::codecs::registry::{RegisterableAudioDecoder, SupportedAudioCodec};
use symphonia::core::codecs::CodecInfo;
use symphonia::core::errors::{decode_error, unsupported_error, Result};
use symphonia::core::packet::PacketRef;

/// Opus output rate.
const RATE: u32 = 48_000;
/// Longest Opus packet: 120 ms at 48 kHz.
const MAX_FRAMES: usize = 5_760;

/// Settings from an `OpusHead` (RFC 7845 §5.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpusHead {
    pub channels: usize,
    /// 0: mono/stereo, 1: Vorbis-order surround (up to 8 channels).
    pub mapping_family: u8,
    /// Output gain in dB.
    pub gain_db: f32,
    /// Frames the encoder put before the audio; dropped from the stream's first packet.
    pub pre_skip: u16,
}

impl OpusHead {
    /// Parses the identification header stored as codec extra data.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 19 || &data[..8] != b"OpusHead" {
            return None;
        }
        let gain = i16::from_le_bytes([data[16], data[17]]);
        Some(Self {
            channels: usize::from(data[9]),
            mapping_family: data[18],
            gain_db: f32::from(gain) / 256.0,
            pre_skip: u16::from_le_bytes([data[10], data[11]]),
        })
    }
}

/// Speaker positions of a Vorbis-order stream (RFC 7845 §5.1.1.2), in stream order.
fn vorbis_positions(channels: usize) -> Option<Vec<Position>> {
    use Position as P;
    Some(match channels {
        1 => vec![P::FRONT_LEFT],
        2 => vec![P::FRONT_LEFT, P::FRONT_RIGHT],
        3 => vec![P::FRONT_LEFT, P::FRONT_CENTER, P::FRONT_RIGHT],
        4 => vec![P::FRONT_LEFT, P::FRONT_RIGHT, P::REAR_LEFT, P::REAR_RIGHT],
        5 => vec![
            P::FRONT_LEFT,
            P::FRONT_CENTER,
            P::FRONT_RIGHT,
            P::REAR_LEFT,
            P::REAR_RIGHT,
        ],
        6 => vec![
            P::FRONT_LEFT,
            P::FRONT_CENTER,
            P::FRONT_RIGHT,
            P::REAR_LEFT,
            P::REAR_RIGHT,
            P::LFE1,
        ],
        7 => vec![
            P::FRONT_LEFT,
            P::FRONT_CENTER,
            P::FRONT_RIGHT,
            P::SIDE_LEFT,
            P::SIDE_RIGHT,
            P::REAR_CENTER,
            P::LFE1,
        ],
        8 => vec![
            P::FRONT_LEFT,
            P::FRONT_CENTER,
            P::FRONT_RIGHT,
            P::SIDE_LEFT,
            P::SIDE_RIGHT,
            P::REAR_LEFT,
            P::REAR_RIGHT,
            P::LFE1,
        ],
        _ => return None,
    })
}

/// For each decoded channel, the Symphonia plane it belongs to (planes follow position bits).
pub fn plane_order(channels: usize) -> Option<Vec<usize>> {
    let positions = vorbis_positions(channels)?;
    Some(
        positions
            .iter()
            .map(|p| positions.iter().filter(|q| q.bits() < p.bits()).count())
            .collect(),
    )
}

/// `rusty-opus` behind Symphonia's decoder interface.
pub struct OpusDecoder {
    params: AudioCodecParameters,
    head: OpusHead,
    gapless: bool,
    inner: OpusMSDecoder,
    planes: Vec<usize>,
    gain: f32,
    pcm: Vec<f32>,
    buf: AudioBuffer<f32>,
}

impl OpusDecoder {
    pub fn try_new(params: &AudioCodecParameters, opts: &AudioDecoderOptions) -> Result<Self> {
        if params.codec != CODEC_ID_OPUS {
            return unsupported_error("opus: not an Opus stream");
        }
        let head = match params.extra_data.as_deref().and_then(OpusHead::parse) {
            Some(head) => head,
            // Without a header only plain mono/stereo can be assumed.
            None => OpusHead {
                channels: params
                    .channels
                    .as_ref()
                    .map_or(2, |c| c.count())
                    .clamp(1, 2),
                mapping_family: 0,
                gain_db: 0.0,
                pre_skip: 0,
            },
        };
        let (Some(positions), Some(planes)) =
            (vorbis_positions(head.channels), plane_order(head.channels))
        else {
            return unsupported_error("opus: unsupported channel count");
        };
        let inner = new_inner(&head)?;
        let mask = positions.iter().fold(Position::empty(), |m, p| m | *p);
        let mut params = params.clone();
        params.sample_rate = Some(RATE);
        params.channels = Some(Channels::Positioned(mask));
        let spec = AudioSpec::new(RATE, Channels::Positioned(mask));
        Ok(Self {
            params,
            head,
            gapless: opts.gapless,
            inner,
            planes,
            gain: 10f32.powf(head.gain_db / 20.0),
            pcm: vec![0.0; MAX_FRAMES * head.channels],
            buf: AudioBuffer::new(spec, MAX_FRAMES),
        })
    }

    fn decode_inner(&mut self, packet: &PacketRef<'_>) -> Result<()> {
        let channels = self.head.channels;
        let frames = match self.inner.decode(packet.data, MAX_FRAMES, &mut self.pcm) {
            Ok(frames) => frames,
            Err(e) => return decode_error(opus_error(&e)),
        };
        self.buf.clear();
        self.buf.render_uninit(Some(frames));
        for (ch, &plane) in self.planes.iter().enumerate() {
            let Some(out) = self.buf.plane_mut(plane) else {
                return decode_error("opus: missing audio plane");
            };
            for (f, sample) in out.iter_mut().enumerate().take(frames) {
                *sample = self.pcm[f * channels + ch] * self.gain;
            }
        }
        if self.gapless {
            // Symphonia's Ogg reader trims the end but leaves the pre-skip to the decoder; it
            // applies to the packet at timestamp 0 (also after rewinding to the start).
            let mut start = packet.trim_start.get() as usize;
            if packet.pts.get() == 0 {
                start = start.max(usize::from(self.head.pre_skip));
            }
            let start = start.min(frames);
            let end = (packet.trim_end.get() as usize).min(frames - start);
            self.buf.trim(start, end);
        }
        Ok(())
    }
}

fn new_inner(head: &OpusHead) -> Result<OpusMSDecoder> {
    let family = i32::from(head.mapping_family.min(1));
    match OpusMSDecoder::new(RATE as i32, head.channels, family) {
        Ok(d) => Ok(d),
        Err(_) => unsupported_error("opus: unsupported channel mapping"),
    }
}

/// Symphonia errors take a `&'static str`; Opus errors are mapped to fixed messages.
fn opus_error(_e: &rusty_opus::Error) -> &'static str {
    "opus: corrupt packet"
}

impl AudioDecoder for OpusDecoder {
    fn reset(&mut self) {
        // `rusty-opus` has no reset; a fresh decoder is cheap and drops all history.
        if let Ok(inner) = new_inner(&self.head) {
            self.inner = inner;
        }
    }

    fn codec_info(&self) -> &CodecInfo {
        &Self::supported_codecs()[0].info
    }

    fn codec_params(&self) -> &AudioCodecParameters {
        &self.params
    }

    fn decode_ref(&mut self, packet: &PacketRef<'_>) -> Result<GenericAudioBufferRef<'_>> {
        if let Err(e) = self.decode_inner(packet) {
            self.buf.clear();
            return Err(e);
        }
        Ok(self.buf.as_generic_audio_buffer_ref())
    }

    fn finalize(&mut self) -> FinalizeResult {
        FinalizeResult::default()
    }

    fn last_decoded(&self) -> GenericAudioBufferRef<'_> {
        self.buf.as_generic_audio_buffer_ref()
    }
}

impl RegisterableAudioDecoder for OpusDecoder {
    fn try_registry_new(
        params: &AudioCodecParameters,
        opts: &AudioDecoderOptions,
    ) -> Result<Box<dyn AudioDecoder>>
    where
        Self: Sized,
    {
        Ok(Box::new(Self::try_new(params, opts)?))
    }

    fn supported_codecs() -> &'static [SupportedAudioCodec] {
        &[SupportedAudioCodec {
            id: CODEC_ID_OPUS,
            info: CodecInfo {
                short_name: "opus",
                long_name: "Opus (rusty-opus)",
                profiles: &[],
            },
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_head_is_parsed() {
        let mut head = b"OpusHead".to_vec();
        head.extend_from_slice(&[1, 6]); // version, channels
        head.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
        head.extend_from_slice(&48_000u32.to_le_bytes());
        head.extend_from_slice(&(-512i16).to_le_bytes()); // -2 dB
        head.extend_from_slice(&[1, 4, 2, 0, 4, 1, 2, 3, 5]); // family 1 + mapping table
        let parsed = OpusHead::parse(&head).unwrap();
        assert_eq!(
            parsed,
            OpusHead {
                channels: 6,
                mapping_family: 1,
                gain_db: -2.0,
                pre_skip: 312,
            }
        );
        assert!(OpusHead::parse(b"OpusTags").is_none());
    }

    #[test]
    fn surround_channels_land_on_symphonia_planes() {
        // Vorbis 5.1 order FL FC FR RL RR LFE -> planes FL FR FC LFE RL RR.
        assert_eq!(plane_order(6).unwrap(), vec![0, 2, 1, 4, 5, 3]);
        assert_eq!(plane_order(2).unwrap(), vec![0, 1]);
        assert!(plane_order(9).is_none());
    }
}
