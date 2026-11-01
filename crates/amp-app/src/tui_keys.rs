//! Player keys of the TUI (TASK-174): Winamp's transport keys, seek, volume, EQ, repeat and
//! shuffle, adding files, folders and links through a prompt with Tab completion, a `?` help
//! overlay, and a confirmation before quitting while music plays.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayApp;
use amp_core_engine::playback::{RepeatMode, ShuffleMode};
use amp_domain::PlaybackState;
use crossterm::event::KeyCode;
use std::path::{Path, MAIN_SEPARATOR};
use std::time::Duration;

/// Every key the player understands, as the help overlay shows them.
pub const KEY_HELP: [(&str, &str); 18] = [
    ("x", "Play"),
    ("c / Space", "Pause / resume"),
    ("v", "Stop"),
    ("z / b", "Previous / next track"),
    ("← / →", "Seek 5 s"),
    ("+ / -", "Volume"),
    ("Enter", "Play the selected track"),
    ("↑ / ↓", "Select a track"),
    ("Delete", "Remove the selected track"),
    ("o", "Add a file, folder or link"),
    ("j or /", "Search"),
    ("[ / ]", "Pick an EQ band"),
    ("{ / }", "EQ band down / up 1 dB"),
    ("e / p", "Show the equalizer / playlist"),
    ("r", "Repeat: off, all, one"),
    ("s", "Shuffle on / off"),
    ("?", "This help"),
    ("q", "Quit (asks while playing)"),
];

/// Seconds the arrow keys seek by.
const SEEK_STEP: u64 = 5;

/// The add prompt: a path or link being typed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PathPrompt {
    pub text: String,
}

/// `input` completed as far as the file system allows: the longest prefix shared by every entry
/// that starts with what was typed, plus a separator when that is a single folder.
#[must_use]
pub fn complete_path(input: &str) -> String {
    let expanded = amp_adapter_storage::expand_tilde(input, None);
    let path = expanded.as_path();
    let typed_dir = input.ends_with(MAIN_SEPARATOR) || input.ends_with('/');
    let (dir, stem) = if typed_dir {
        (path.to_path_buf(), String::new())
    } else {
        let Some(dir) = path.parent() else {
            return input.to_string();
        };
        let stem = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        (dir.to_path_buf(), stem)
    };
    let Ok(entries) = std::fs::read_dir(if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        &dir
    }) else {
        return input.to_string();
    };
    let mut matches: Vec<(String, bool)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.starts_with(&stem).then(|| (name, e.path().is_dir()))
        })
        .collect();
    matches.sort();
    let Some((first, _)) = matches.first() else {
        return input.to_string();
    };
    let mut common = first.clone();
    for (name, _) in &matches[1..] {
        let shared = common
            .chars()
            .zip(name.chars())
            .take_while(|(a, b)| a == b)
            .count();
        common = common.chars().take(shared).collect();
    }
    // A single folder, or a name that already is a folder: add the separator so the next Tab
    // looks inside.
    let unique_dir = (matches.len() == 1 && matches[0].1)
        || (common == stem && !stem.is_empty() && dir.join(&stem).is_dir());
    if common == stem && !unique_dir {
        return input.to_string();
    }
    let prefix = &input[..input.len() - stem.len()];
    let mut out = format!("{prefix}{common}");
    if unique_dir {
        out.push(MAIN_SEPARATOR);
    }
    out
}

impl MultiplayApp {
    /// Keys while the add prompt is open. Returns `true` when the key was used.
    pub(crate) fn handle_prompt_key(&mut self, key: KeyCode) -> bool {
        let Some(prompt) = self.prompt.as_mut() else {
            return false;
        };
        match key {
            KeyCode::Esc => self.prompt = None,
            KeyCode::Tab => prompt.text = complete_path(&prompt.text),
            KeyCode::Backspace => {
                prompt.text.pop();
            }
            KeyCode::Char(c) => prompt.text.push(c),
            KeyCode::Enter => {
                let text = std::mem::take(&mut prompt.text);
                self.prompt = None;
                self.add_from_prompt(text.trim());
            }
            _ => {}
        }
        true
    }

    /// Adds a link, or the file or folder at `text`, and says what happened in the status line.
    fn add_from_prompt(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(track) = amp_domain::source_url::track_from_link(text) {
            self.playlist.push(track);
            self.status = Some("Added 1 link".into());
            return;
        }
        let path = amp_adapter_storage::expand_tilde(text, None);
        if !path.exists() {
            self.status = Some(format!("Not found: {}", path.display()));
            return;
        }
        let factory = self.audio_driver.as_ref().map(|d| d.decoder_factory());
        let fallback = move |p: &Path| {
            let dec = factory.as_ref()?.open_file(&p.to_string_lossy()).ok()?;
            Some(amp_adapter_storage::file_name_track(
                p,
                dec.duration(),
                Some(dec.sample_rate()),
            ))
        };
        let report = amp_adapter_storage::import::import_paths(&[path], &fallback);
        let added = report.tracks.len();
        self.playlist.extend(report.tracks);
        let skipped: usize = report.skipped.values().sum();
        self.status = Some(match (added, skipped) {
            (n, 0) => format!("Added {n} track{}", if n == 1 { "" } else { "s" }),
            (n, s) => format!("Added {n}, skipped {s} that cannot play yet"),
        });
    }

    /// Plays playlist entry `idx` and queues the next one for gapless playback.
    pub(crate) fn play_index(&mut self, idx: usize) {
        let Some(track) = self.playlist.get(idx).cloned() else {
            return;
        };
        self.selected_playlist_idx = idx;
        self.current_title = format!(
            "{}. {} - {}",
            idx + 1,
            track.metadata.artist,
            track.metadata.title
        );
        self.playback_state = PlaybackState::Playing;
        if let Some(driver) = &self.audio_driver {
            driver.play(track);
            driver.set_next_track(self.playlist.get(idx + 1).cloned());
        }
    }

    /// Player keys outside the search, prompt and help overlays. Returns `true` to quit.
    pub(crate) fn handle_player_key(&mut self, key: KeyCode) -> bool {
        let driver = self.audio_driver.clone();
        match key {
            KeyCode::Char('q') => {
                if self.playback_state == PlaybackState::Playing {
                    self.confirm_quit = true;
                } else {
                    return true;
                }
            }
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('o') => self.prompt = Some(PathPrompt::default()),
            KeyCode::Char('x') => self.play_index(self.selected_playlist_idx),
            KeyCode::Char('c') | KeyCode::Char(' ') => match self.playback_state {
                PlaybackState::Playing => {
                    self.playback_state = PlaybackState::Paused;
                    if let Some(d) = &driver {
                        d.pause();
                    }
                }
                PlaybackState::Paused => {
                    self.playback_state = PlaybackState::Playing;
                    if let Some(d) = &driver {
                        d.resume();
                    }
                }
                _ => self.play_index(self.selected_playlist_idx),
            },
            KeyCode::Char('v') => {
                self.playback_state = PlaybackState::Stopped;
                self.elapsed = Duration::ZERO;
                if let Some(d) = &driver {
                    d.stop();
                }
            }
            KeyCode::Char('z') if self.selected_playlist_idx > 0 => {
                self.play_index(self.selected_playlist_idx - 1);
            }
            KeyCode::Char('b') if self.selected_playlist_idx + 1 < self.playlist.len() => {
                self.play_index(self.selected_playlist_idx + 1);
            }
            KeyCode::Left | KeyCode::Right => {
                if let Some(d) = &driver {
                    let step = Duration::from_secs(SEEK_STEP);
                    let now = *d.elapsed.lock().unwrap();
                    let total = *d.total_duration.lock().unwrap();
                    let target = if key == KeyCode::Left {
                        now.saturating_sub(step)
                    } else {
                        let t = now + step;
                        total.map_or(t, |total| t.min(total))
                    };
                    d.seek(target);
                }
            }
            KeyCode::Delete | KeyCode::Backspace if !self.playlist.is_empty() => {
                self.playlist.remove(self.selected_playlist_idx);
                self.selected_playlist_idx = self
                    .selected_playlist_idx
                    .min(self.playlist.len().saturating_sub(1));
            }
            KeyCode::Char('[') => self.eq_band = self.eq_band.saturating_sub(1),
            KeyCode::Char(']') => self.eq_band = (self.eq_band + 1).min(9),
            KeyCode::Char('{') | KeyCode::Char('}') => {
                let delta = if key == KeyCode::Char('}') { 1.0 } else { -1.0 };
                let band = self.eq_band;
                self.eq_gains[band] = (self.eq_gains[band] + delta).clamp(-12.0, 12.0);
                if let Some(d) = &driver {
                    d.set_eq_band(band, self.eq_gains[band]);
                }
            }
            KeyCode::Char('r') => {
                if let Some(d) = &driver {
                    let next = match d.repeat_mode() {
                        RepeatMode::Off => RepeatMode::RepeatAll,
                        RepeatMode::RepeatAll => RepeatMode::RepeatOne,
                        RepeatMode::RepeatOne => RepeatMode::Off,
                    };
                    d.set_repeat_mode(next);
                    self.status = Some(format!("Repeat: {next:?}"));
                }
            }
            KeyCode::Char('s') => {
                if let Some(d) = &driver {
                    let next = if d.shuffle_mode() == ShuffleMode::Off {
                        ShuffleMode::TrueRandom
                    } else {
                        ShuffleMode::Off
                    };
                    d.set_shuffle_mode(next);
                    self.status = Some(format!(
                        "Shuffle: {}",
                        if next == ShuffleMode::Off {
                            "off"
                        } else {
                            "on"
                        }
                    ));
                }
            }
            _ => return self.handle_classic_key(key),
        }
        false
    }
}

impl MultiplayApp {
    /// Tells the system what plays and turns media keys into the matching player keys.
    pub fn sync_media_controls(&mut self) {
        use amp_ports::{MediaKeyEvent, MediaPlayback, NowPlaying};
        let (Some(mut controls), Some(driver)) =
            (self.media_controls.take(), self.audio_driver.clone())
        else {
            return;
        };
        let track = driver.current_track.lock().ok().and_then(|t| t.clone());
        if let Some(track) = &track {
            controls.set_now_playing(&NowPlaying {
                title: track.metadata.title.clone(),
                artist: track.metadata.artist.clone(),
                album: track.metadata.album.clone(),
                duration: *driver.total_duration.lock().unwrap(),
                cover_url: None,
            });
        }
        let position = *driver.elapsed.lock().unwrap();
        controls.set_playback(match self.playback_state {
            PlaybackState::Playing => MediaPlayback::Playing { position },
            PlaybackState::Paused => MediaPlayback::Paused { position },
            _ => MediaPlayback::Stopped,
        });
        let events = controls.take_events();
        self.media_controls = Some(controls);
        for event in events {
            let key = match event {
                MediaKeyEvent::Play if self.playback_state == PlaybackState::Playing => continue,
                MediaKeyEvent::Pause if self.playback_state != PlaybackState::Playing => continue,
                MediaKeyEvent::Play | MediaKeyEvent::Pause | MediaKeyEvent::Toggle => {
                    KeyCode::Char(' ')
                }
                MediaKeyEvent::Stop => KeyCode::Char('v'),
                MediaKeyEvent::Next => KeyCode::Char('b'),
                MediaKeyEvent::Previous => KeyCode::Char('z'),
                MediaKeyEvent::SeekStep { forward: true } => KeyCode::Right,
                MediaKeyEvent::SeekBy(secs) if secs > 0 => KeyCode::Right,
                MediaKeyEvent::SeekStep { .. } | MediaKeyEvent::SeekBy(_) => KeyCode::Left,
                MediaKeyEvent::SetPosition(at) => {
                    driver.seek(at);
                    continue;
                }
            };
            self.handle_player_key(key);
        }
    }
}
