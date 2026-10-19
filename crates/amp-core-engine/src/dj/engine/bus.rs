//! Output routing for the master and headphone (cue) buses.
//!
//! The headphones hear `cue × (1 − cue mix) + master × cue mix`, scaled by the cue volume. Where
//! that goes depends on the hardware:
//! - [`CueRouting::SecondDevice`]: master on the main device, headphones on another device;
//! - [`CueRouting::ExtraChannels`]: one 4-channel DJ interface, master on 1/2, headphones on 3/4;
//! - [`CueRouting::Split`]: one stereo device and a splitter cable, master mono on the left and
//!   headphones mono on the right;
//! - [`CueRouting::Off`]: master only (no cue available).

#![forbid(unsafe_code)]

/// Where the headphone mix goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueRouting {
    /// No headphone output.
    Off,
    /// Master mono on the left, headphones mono on the right of the main device.
    Split,
    /// Headphones on channels 3/4 of the main device.
    ExtraChannels,
    /// Headphones on a separate output device.
    SecondDevice,
}

impl CueRouting {
    pub(crate) fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Split,
            2 => Self::ExtraChannels,
            3 => Self::SecondDevice,
            _ => Self::Off,
        }
    }
}

/// Headphone mix settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CueMix {
    /// 0 = cue only, 1 = master only.
    pub mix: f32,
    /// Headphone level, 0 to 1.
    pub volume: f32,
}

/// Writes the main device's frames (`main_channels` interleaved channels) and, for
/// [`CueRouting::SecondDevice`], the stereo headphone frames into `phones`.
///
/// `master` and `cue` are interleaved stereo of equal length; `main` must hold
/// `frames × main_channels` samples and `phones` `frames × 2`. Real-time safe.
pub fn route(
    routing: CueRouting,
    master: &[f32],
    cue: &[f32],
    mix: CueMix,
    main_channels: usize,
    main: &mut [f32],
    phones: &mut [f32],
) {
    let ch = main_channels.max(1);
    let routing = match routing {
        CueRouting::ExtraChannels if ch < 4 => CueRouting::Off,
        r => r,
    };
    let (m, v) = (mix.mix.clamp(0.0, 1.0), mix.volume.clamp(0.0, 1.0));
    let frames = (master.len().min(cue.len()) / 2).min(main.len() / ch);
    main.fill(0.0);
    for f in 0..frames {
        let (ml, mr) = (master[f * 2], master[f * 2 + 1]);
        let pl = (cue[f * 2] * (1.0 - m) + ml * m) * v;
        let pr = (cue[f * 2 + 1] * (1.0 - m) + mr * m) * v;
        let out = &mut main[f * ch..(f + 1) * ch];
        match (routing, ch) {
            (_, 1) => out[0] = 0.5 * (ml + mr),
            (CueRouting::Split, _) => {
                out[0] = 0.5 * (ml + mr);
                out[1] = 0.5 * (pl + pr);
            }
            (CueRouting::ExtraChannels, _) => {
                out[..4].copy_from_slice(&[ml, mr, pl, pr]);
            }
            _ => {
                out[0] = ml;
                out[1] = mr;
            }
        }
        if routing == CueRouting::SecondDevice && phones.len() >= (f + 1) * 2 {
            phones[f * 2] = pl;
            phones[f * 2 + 1] = pr;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: [f32; 4] = [0.4, 0.2, 0.4, 0.2];
    const CUE: [f32; 4] = [-0.6, -0.2, -0.6, -0.2];
    const ONLY_CUE: CueMix = CueMix {
        mix: 0.0,
        volume: 1.0,
    };

    fn run(routing: CueRouting, ch: usize, mix: CueMix) -> (Vec<f32>, Vec<f32>) {
        let mut main = vec![9.0; 2 * ch];
        let mut phones = vec![0.0; 4];
        route(routing, &MASTER, &CUE, mix, ch, &mut main, &mut phones);
        (main, phones)
    }

    #[test]
    fn dj_cue_split_puts_master_left_and_cue_right_in_mono() {
        let (main, _) = run(CueRouting::Split, 2, ONLY_CUE);
        assert_eq!(main, vec![0.3, -0.4, 0.3, -0.4]);
    }

    #[test]
    fn dj_cue_extra_channels_drive_3_and_4_of_a_dj_interface() {
        let (main, _) = run(CueRouting::ExtraChannels, 4, ONLY_CUE);
        assert_eq!(&main[..4], &[0.4, 0.2, -0.6, -0.2]);
    }

    #[test]
    fn dj_cue_extra_channels_fall_back_to_master_on_stereo_devices() {
        let (main, _) = run(CueRouting::ExtraChannels, 2, ONLY_CUE);
        assert_eq!(main, MASTER.to_vec());
    }

    #[test]
    fn dj_cue_second_device_gets_the_headphone_mix() {
        let (main, phones) = run(CueRouting::SecondDevice, 2, ONLY_CUE);
        assert_eq!(main, MASTER.to_vec());
        assert_eq!(phones, CUE.to_vec());
    }

    #[test]
    fn dj_cue_mix_and_volume_shape_the_headphones() {
        let half = CueMix {
            mix: 0.5,
            volume: 0.5,
        };
        let (_, phones) = run(CueRouting::SecondDevice, 2, half);
        let want = [(-0.6 + 0.4) * 0.25, 0.0];
        assert!((phones[0] - want[0]).abs() < 1e-6 && (phones[1] - want[1]).abs() < 1e-6);
    }

    #[test]
    fn dj_cue_off_plays_master_and_silences_extra_channels() {
        let (main, _) = run(CueRouting::Off, 4, ONLY_CUE);
        assert_eq!(&main[..4], &[0.4, 0.2, 0.0, 0.0]);
        let (mono, _) = run(CueRouting::Split, 1, ONLY_CUE);
        assert!((mono[0] - 0.3).abs() < 1e-6, "mono device gets master");
    }
}
