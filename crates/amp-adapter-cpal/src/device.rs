//! Hardware audio device resolution and capability negotiation for CPAL (`TASK-132`).

#![forbid(unsafe_code)]

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{BufferSize, Device, SampleFormat, StreamConfig, SupportedBufferSize};

/// Default hardware buffer size in frames when the OS driver reports `SupportedBufferSize::Unknown`.
pub const DEFAULT_DEVICE_BUFFER_FRAMES: u32 = 512;

/// Negotiated hardware sample format supported by the real-time conversion + TPDF dither path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiatedSampleFormat {
    F32,
    I32,
    I16,
    U16,
}

impl NegotiatedSampleFormat {
    /// Preference rank (higher is better).
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::F32 => 4,
            Self::I32 => 3,
            Self::I16 => 2,
            Self::U16 => 1,
        }
    }

    /// Maps a `cpal::SampleFormat` into a supported [`NegotiatedSampleFormat`].
    #[must_use]
    pub const fn from_cpal(fmt: SampleFormat) -> Option<Self> {
        match fmt {
            SampleFormat::F32 => Some(Self::F32),
            SampleFormat::I32 => Some(Self::I32),
            SampleFormat::I16 => Some(Self::I16),
            SampleFormat::U16 => Some(Self::U16),
            _ => None,
        }
    }
}

/// Hardware-independent representation of a CPAL supported output config range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedConfigRangeSpec {
    pub channels: u16,
    pub min_sample_rate: u32,
    pub max_sample_rate: u32,
    pub sample_format: NegotiatedSampleFormat,
    pub buffer_frames: Option<u32>,
}

/// Result of negotiating sample format, channel count, sample rate, and hardware buffer frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedStreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: NegotiatedSampleFormat,
    pub buffer_frames: u32,
}

/// Description of an audio hardware output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceDescription {
    pub name: String,
    pub is_default: bool,
}

/// Pure negotiation algorithm (`TASK-132`):
/// 1. Prefers stereo (`channels == 2`), else multichannel (`> 2`, smallest count first), else mono (`1`).
/// 2. Prefers `preferred_rate` when supported (`bit_perfect`), else `default_cfg` rate, else `48_000`/`44_100`.
/// 3. Prefers `F32 > I32 > I16 > U16`.
#[must_use]
pub fn negotiate_stream_config(
    ranges: &[SupportedConfigRangeSpec],
    default_cfg: Option<NegotiatedStreamConfig>,
    preferred_rate: Option<u32>,
) -> NegotiatedStreamConfig {
    let default_rate = default_cfg.as_ref().map_or(48_000, |d| d.sample_rate);
    let mut best: Option<((u8, i32, u8, u8), NegotiatedStreamConfig)> = None;

    for r in ranges {
        if r.channels == 0 {
            continue;
        }
        let ch_tier: u8 = if r.channels == 2 {
            3
        } else if r.channels > 2 {
            2
        } else {
            1
        };
        let ch_penalty: i32 = if r.channels > 2 {
            -i32::from(r.channels)
        } else {
            0
        };

        let (rate_tier, chosen_rate) = if let Some(pref) = preferred_rate {
            if pref >= r.min_sample_rate && pref <= r.max_sample_rate {
                (3u8, pref)
            } else if default_rate >= r.min_sample_rate && default_rate <= r.max_sample_rate {
                (2u8, default_rate)
            } else if 48_000 >= r.min_sample_rate && 48_000 <= r.max_sample_rate {
                (1u8, 48_000)
            } else {
                (0u8, pref.clamp(r.min_sample_rate, r.max_sample_rate))
            }
        } else if default_rate >= r.min_sample_rate && default_rate <= r.max_sample_rate {
            (3u8, default_rate)
        } else if 48_000 >= r.min_sample_rate && 48_000 <= r.max_sample_rate {
            (2u8, 48_000)
        } else if 44_100 >= r.min_sample_rate && 44_100 <= r.max_sample_rate {
            (1u8, 44_100)
        } else {
            (
                0u8,
                default_rate.clamp(r.min_sample_rate, r.max_sample_rate),
            )
        };

        let score = (ch_tier, ch_penalty, rate_tier, r.sample_format.rank());
        let candidate = NegotiatedStreamConfig {
            channels: r.channels,
            sample_rate: chosen_rate,
            sample_format: r.sample_format,
            buffer_frames: r.buffer_frames.unwrap_or(DEFAULT_DEVICE_BUFFER_FRAMES),
        };

        if best.as_ref().is_none_or(|(b_score, _)| score > *b_score) {
            best = Some((score, candidate));
        }
    }

    if let Some((_, cfg)) = best {
        cfg
    } else if let Some(def) = default_cfg {
        def
    } else {
        NegotiatedStreamConfig {
            channels: 2,
            sample_rate: preferred_rate.unwrap_or(48_000),
            sample_format: NegotiatedSampleFormat::F32,
            buffer_frames: DEFAULT_DEVICE_BUFFER_FRAMES,
        }
    }
}

/// Returns the current system default output device name, if any.
#[must_use]
pub fn default_output_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|d| device_name(&d).ok())
}

/// Lists all detected hardware audio output devices on the host.
pub fn list_output_devices() -> Vec<AudioDeviceDescription> {
    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .and_then(|d| device_name(&d).ok());
    let mut result = Vec::new();

    if let Ok(devices) = host.devices() {
        for d in devices {
            if let Ok(name) = device_name(&d) {
                let lower = name.to_lowercase();
                if lower.contains("microphone") || lower.contains("input") {
                    continue;
                }
                let is_default = default_name.as_ref() == Some(&name);
                if !result
                    .iter()
                    .any(|item: &AudioDeviceDescription| item.name == name)
                {
                    result.push(AudioDeviceDescription { name, is_default });
                }
            }
        }
    }
    result
}

/// Resolves a requested device by name (or default output device) and negotiates a working `StreamConfig`.
pub fn resolve_output_device(
    target_name: Option<&str>,
) -> Result<(Device, StreamConfig, String), String> {
    let (dev, cfg, _fmt, _buf_frames, name) = resolve_output_device_with_rate(target_name, None)?;
    Ok((dev, cfg, name))
}

/// Resolves an output device and negotiates sample format, channel count, sample rate, and buffer frames.
pub fn resolve_output_device_with_rate(
    target_name: Option<&str>,
    preferred_rate: Option<u32>,
) -> Result<(Device, StreamConfig, NegotiatedSampleFormat, u32, String), String> {
    let host = cpal::default_host();

    let (device, resolved_name) = if let Some(target) = target_name {
        let all_devs = host
            .devices()
            .map_err(|e| format!("Device enumeration failed: {e}"))?;
        let mut found = None;
        for d in all_devs {
            if let Ok(name) = device_name(&d) {
                if name == target {
                    found = Some((d, name));
                    break;
                }
            }
        }
        found.ok_or_else(|| format!("Target audio device '{target}' not found"))?
    } else if let Some(def) = host.default_output_device() {
        let name = device_name(&def).unwrap_or_else(|_| "Default Audio Device".to_string());
        (def, name)
    } else {
        let mut out_devs = host
            .output_devices()
            .map_err(|e| format!("Output device enumeration failed: {e}"))?;
        let d = out_devs
            .next()
            .ok_or_else(|| "No hardware audio output devices found".to_string())?;
        let name = device_name(&d).unwrap_or_else(|_| "Unknown Audio Device".to_string());
        (d, name)
    };

    let negotiated = negotiate_for_device(&device, preferred_rate);
    let config = StreamConfig {
        channels: negotiated.channels,
        sample_rate: negotiated.sample_rate,
        buffer_size: BufferSize::Default,
    };

    Ok((
        device,
        config,
        negotiated.sample_format,
        negotiated.buffer_frames,
        resolved_name,
    ))
}

/// Negotiates the best [`NegotiatedStreamConfig`] for an already-opened [`Device`].
pub fn negotiate_for_device(
    device: &Device,
    preferred_rate: Option<u32>,
) -> NegotiatedStreamConfig {
    let default_cfg = device.default_output_config().ok().map(|sup| {
        let fmt = NegotiatedSampleFormat::from_cpal(sup.sample_format())
            .unwrap_or(NegotiatedSampleFormat::F32);
        let buf_frames = extract_buffer_frames(sup.buffer_size());
        NegotiatedStreamConfig {
            channels: sup.channels(),
            sample_rate: sup.sample_rate(),
            sample_format: fmt,
            buffer_frames: buf_frames.unwrap_or(DEFAULT_DEVICE_BUFFER_FRAMES),
        }
    });

    let mut ranges = Vec::new();
    if let Ok(iter) = device.supported_output_configs() {
        for r in iter {
            if let Some(fmt) = NegotiatedSampleFormat::from_cpal(r.sample_format()) {
                ranges.push(SupportedConfigRangeSpec {
                    channels: r.channels(),
                    min_sample_rate: r.min_sample_rate(),
                    max_sample_rate: r.max_sample_rate(),
                    sample_format: fmt,
                    buffer_frames: extract_buffer_frames(r.buffer_size()),
                });
            }
        }
    }

    negotiate_stream_config(&ranges, default_cfg, preferred_rate)
}

fn extract_buffer_frames(bs: &SupportedBufferSize) -> Option<u32> {
    match bs {
        SupportedBufferSize::Range { min, max } => {
            Some(DEFAULT_DEVICE_BUFFER_FRAMES.clamp(*min, *max))
        }
        SupportedBufferSize::Unknown => None,
    }
}

/// The device's human-readable name (cpal 0.18 moved it into the device description).
fn device_name(device: &Device) -> Result<String, cpal::Error> {
    device.description().map(|d| d.name().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_negotiation_prefers_f32_stereo_and_preferred_rate() {
        let ranges = vec![
            SupportedConfigRangeSpec {
                channels: 2,
                min_sample_rate: 44_100,
                max_sample_rate: 96_000,
                sample_format: NegotiatedSampleFormat::I16,
                buffer_frames: Some(256),
            },
            SupportedConfigRangeSpec {
                channels: 2,
                min_sample_rate: 44_100,
                max_sample_rate: 192_000,
                sample_format: NegotiatedSampleFormat::F32,
                buffer_frames: Some(512),
            },
        ];

        let cfg = negotiate_stream_config(&ranges, None, Some(96_000));
        assert_eq!(cfg.channels, 2);
        assert_eq!(cfg.sample_rate, 96_000);
        assert_eq!(cfg.sample_format, NegotiatedSampleFormat::F32);
        assert_eq!(cfg.buffer_frames, 512);
    }

    #[test]
    fn format_negotiation_falls_back_to_integer_dac_and_8ch_hdmi() {
        // USB DAC offering only I32 and I16 stereo
        let usb_dac = vec![
            SupportedConfigRangeSpec {
                channels: 2,
                min_sample_rate: 44_100,
                max_sample_rate: 48_000,
                sample_format: NegotiatedSampleFormat::I16,
                buffer_frames: Some(512),
            },
            SupportedConfigRangeSpec {
                channels: 2,
                min_sample_rate: 44_100,
                max_sample_rate: 96_000,
                sample_format: NegotiatedSampleFormat::I32,
                buffer_frames: Some(512),
            },
        ];
        let dac_cfg = negotiate_stream_config(&usb_dac, None, Some(48_000));
        assert_eq!(dac_cfg.sample_format, NegotiatedSampleFormat::I32);
        assert_eq!(dac_cfg.channels, 2);

        // HDMI output offering only 8-channel I16 and mono F32
        let hdmi = vec![
            SupportedConfigRangeSpec {
                channels: 1,
                min_sample_rate: 48_000,
                max_sample_rate: 48_000,
                sample_format: NegotiatedSampleFormat::F32,
                buffer_frames: Some(1024),
            },
            SupportedConfigRangeSpec {
                channels: 8,
                min_sample_rate: 48_000,
                max_sample_rate: 48_000,
                sample_format: NegotiatedSampleFormat::I16,
                buffer_frames: Some(1024),
            },
        ];
        let hdmi_cfg = negotiate_stream_config(&hdmi, None, None);
        assert_eq!(hdmi_cfg.channels, 8);
        assert_eq!(hdmi_cfg.sample_format, NegotiatedSampleFormat::I16);
    }
}
