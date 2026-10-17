//! Hardware audio device resolution and capability negotiation for CPAL.

#![forbid(unsafe_code)]

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{BufferSize, Device, SampleRate, StreamConfig};

/// Description of an audio hardware output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceDescription {
    pub name: String,
    pub is_default: bool,
}

/// Lists all detected hardware audio output devices on the host.
pub fn list_output_devices() -> Vec<AudioDeviceDescription> {
    let host = cpal::default_host();
    let default_name = host.default_output_device().and_then(|d| d.name().ok());
    let mut result = Vec::new();

    if let Ok(devices) = host.devices() {
        for d in devices {
            if let Ok(name) = d.name() {
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
    let host = cpal::default_host();

    let (device, resolved_name) = if let Some(target) = target_name {
        let all_devs = host
            .devices()
            .map_err(|e| format!("Device enumeration failed: {e}"))?;
        let mut found = None;
        for d in all_devs {
            if let Ok(name) = d.name() {
                if name == target {
                    found = Some((d, name));
                    break;
                }
            }
        }
        found.ok_or_else(|| format!("Target audio device '{target}' not found"))?
    } else if let Some(def) = host.default_output_device() {
        let name = def
            .name()
            .unwrap_or_else(|_| "Default Audio Device".to_string());
        (def, name)
    } else {
        let mut out_devs = host
            .output_devices()
            .map_err(|e| format!("Output device enumeration failed: {e}"))?;
        let d = out_devs
            .next()
            .ok_or_else(|| "No hardware audio output devices found".to_string())?;
        let name = d
            .name()
            .unwrap_or_else(|_| "Unknown Audio Device".to_string());
        (d, name)
    };

    // Determine working StreamConfig, handling macOS CoreAudio Apple Silicon driver quirks
    let config = if let Ok(supported) = device.default_output_config() {
        StreamConfig {
            channels: supported.channels(),
            sample_rate: supported.sample_rate(),
            buffer_size: BufferSize::Default,
        }
    } else {
        // Fallback standard 48kHz stereo configuration for built-in Mac speakers
        StreamConfig {
            channels: 2,
            sample_rate: SampleRate(48000),
            buffer_size: BufferSize::Default,
        }
    };

    Ok((device, config, resolved_name))
}
