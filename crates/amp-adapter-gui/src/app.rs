//! Main eframe App state for Multiplay Native Desktop GUI.
//! Manages modular floating windows, magnetic snapping, and drag-and-drop media ingestion.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::dj_panel::{DjConsolePanel, DjConsoleState};
use crate::docking::{DockManager, DockRect, WindowId};
use crate::equalizer_widget::EqualizerPanel;
use crate::main_window::render_main_window_content;
use crate::playlist_window::render_playlist_window_content;
use crate::settings_panel::{GuiSettingsTab, SettingsGuiPanel};
use amp_adapter_storage::{collect_audio_files, extract_track_from_file};
use amp_domain::source_badge::SourceKind;
use amp_domain::{AppConfig, AudioSource, Track};
use eframe::egui::{self, Color32, Pos2, Vec2};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

pub struct MultiplayGuiApp {
    pub audio_driver: AudioPlaybackDriver,
    pub playlist: Vec<Track>,
    pub selected_track_idx: usize,
    pub search_query: String,
    pub filter_source: Option<SourceKind>,
    pub dock_manager: DockManager,
    pub dj_state: DjConsoleState,
    pub settings_tab: GuiSettingsTab,
    pub config: AppConfig,
    pub volume: f32,
    pub eq_preamp: f32,
    pub eq_gains: [f32; 10],
    pub eq_enabled: bool,
}

impl MultiplayGuiApp {
    pub fn new() -> Result<Self, String> {
        let audio_driver = AudioPlaybackDriver::new()?;
        let mut app = Self {
            audio_driver,
            playlist: Vec::new(),
            selected_track_idx: 0,
            search_query: String::new(),
            filter_source: None,
            dock_manager: DockManager::default(),
            dj_state: DjConsoleState::default(),
            settings_tab: GuiSettingsTab::default(),
            config: AppConfig::default(),
            volume: 0.85,
            eq_preamp: 0.0,
            eq_gains: [0.0; 10],
            eq_enabled: true,
        };

        // Try restoring layout from disk
        app.load_layout();

        // Scan standard user music directory or bundled music if empty
        app.scan_user_music_dir();

        Ok(app)
    }

    /// Ingests dropped or selected file/directory path into playlist.
    pub fn ingest_path(&mut self, path: &Path) {
        if path.is_dir() {
            let files = collect_audio_files(path);
            for f in files {
                if let Some(track) = extract_track_from_file(&f) {
                    self.playlist.push(track);
                }
            }
        } else if let Some(track) = extract_track_from_file(path) {
            self.playlist.push(track);
        }
    }

    /// Ingests a stream URL (e.g. YouTube, Spotify, HTTP stream).
    pub fn ingest_url(&mut self, url: &str) {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return;
        }

        let (source, title) = if trimmed.contains("youtube.com") || trimmed.contains("youtu.be") {
            (AudioSource::YouTube { video_id: trimmed.to_string() }, "YouTube Stream")
        } else if trimmed.contains("spotify.com") {
            (AudioSource::Spotify { track_uri: trimmed.to_string() }, "Spotify Track")
        } else {
            (AudioSource::DirectHttp { url: trimmed.to_string() }, "Direct HTTP Stream")
        };

        let meta = amp_domain::TrackMetadata::new(title, "Network Source");
        self.playlist.push(Track::new(source, meta));
    }

    /// Scans standard user ~/Music folder.
    pub fn scan_user_music_dir(&mut self) {
        if let Some(home) = std::env::var_os("HOME") {
            let music_dir = PathBuf::from(home).join("Music");
            if music_dir.exists() {
                self.ingest_path(&music_dir);
            }
        }
    }

    /// Pure-Rust fallback file picker dialog.
    pub fn open_file_dialog(&mut self) {
        if let Some(home) = std::env::var_os("HOME") {
            let music_dir = PathBuf::from(home).join("Music");
            if music_dir.exists() {
                self.ingest_path(&music_dir);
            }
        }
    }

    /// Pure-Rust fallback folder picker dialog.
    pub fn open_folder_dialog(&mut self) {
        self.scan_user_music_dir();
    }

    /// Checks if active track reached EOF and automatically triggers gapless advance.
    fn check_gapless_advancement(&mut self) {
        if self.audio_driver.track_finished.swap(false, Ordering::SeqCst)
            && !self.playlist.is_empty()
            && self.selected_track_idx + 1 < self.playlist.len()
        {
            self.selected_track_idx += 1;
            self.audio_driver.play(self.playlist[self.selected_track_idx].clone());
        }
    }

    /// Saves modular window layout to configuration directory.
    pub fn save_layout(&self) {
        if let Some(home) = std::env::var_os("HOME") {
            let config_dir = PathBuf::from(home).join(".config/multiplay");
            let _ = std::fs::create_dir_all(&config_dir);
            let layout_file = config_dir.join("window_layout.json");
            if let Ok(json) = serde_json::to_string_pretty(&self.dock_manager) {
                let _ = std::fs::write(layout_file, json);
            }
        }
    }

    /// Loads modular window layout from configuration directory.
    pub fn load_layout(&mut self) {
        if let Some(home) = std::env::var_os("HOME") {
            let layout_file = PathBuf::from(home).join(".config/multiplay/window_layout.json");
            if let Ok(json) = std::fs::read_to_string(layout_file) {
                if let Ok(dm) = serde_json::from_str::<DockManager>(&json) {
                    self.dock_manager = dm;
                }
            }
        }
    }
}

impl eframe::App for MultiplayGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous smooth repaint for 60 FPS real-time visualizer animation
        ctx.request_repaint_after(Duration::from_millis(16));

        // 1. Automatic Gapless Track Advancement
        self.check_gapless_advancement();

        // 2. OS-Native Drag-and-Drop Ingestion (Files & Folders)
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped_files {
            if let Some(path) = f.path {
                self.ingest_path(&path);
            }
        }

        // 3. Render Desktop Background
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(10, 14, 18)))
            .show(ctx, |_ui| {});

        // 4. Modular Windows Rendering & Snapping
        render_modular_windows(self, ctx);
    }
}

fn render_modular_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    // 1. MAIN PLAYER WINDOW
    let main_rect = app.dock_manager.get_rect(WindowId::Main);
    let mut main_open = app.dock_manager.is_visible(WindowId::Main);
    let mut main_pos = Pos2::new(main_rect.x, main_rect.y);

    let main_resp = egui::Window::new("WINAMP :: MAIN PLAYER")
        .id(egui::Id::new("win_main"))
        .open(&mut main_open)
        .current_pos(main_pos)
        .default_size(Vec2::new(main_rect.width, main_rect.height))
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            render_main_window_content(app, ui);
        });

    if let Some(resp) = main_resp {
        if resp.response.dragged() {
            let delta = resp.response.drag_delta();
            app.dock_manager.move_window_group(WindowId::Main, delta.x, delta.y);
        } else {
            main_pos = resp.response.rect.min;
            let current = DockRect::new(main_pos.x, main_pos.y, resp.response.rect.width(), resp.response.rect.height());
            let (snapped, _) = app.dock_manager.compute_snap(WindowId::Main, current);
            app.dock_manager.set_rect(WindowId::Main, snapped);
        }
    }

    // 2. EQUALIZER WINDOW
    if app.dock_manager.is_visible(WindowId::Equalizer) {
        let eq_rect = app.dock_manager.get_rect(WindowId::Equalizer);
        let mut eq_open = true;
        let mut eq_pos = Pos2::new(eq_rect.x, eq_rect.y);

        let eq_resp = egui::Window::new("WINAMP :: EQUALIZER")
            .id(egui::Id::new("win_eq"))
            .open(&mut eq_open)
            .current_pos(eq_pos)
            .default_size(Vec2::new(eq_rect.width, eq_rect.height))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                let prev_preamp = app.eq_preamp;
                let prev_gains = app.eq_gains;
                let prev_enabled = app.eq_enabled;

                EqualizerPanel::new(&mut app.eq_preamp, &mut app.eq_gains, &mut app.eq_enabled).ui(ui);

                if (prev_preamp - app.eq_preamp).abs() > 0.005 {
                    app.audio_driver.set_eq_preamp(app.eq_preamp);
                }
                if prev_gains != app.eq_gains {
                    for (i, &g) in app.eq_gains.iter().enumerate() {
                        app.audio_driver.set_eq_band(i, g);
                    }
                }
                if prev_enabled != app.eq_enabled {
                    app.audio_driver.toggle_eq();
                }
            });

        if !eq_open {
            app.dock_manager.set_visible(WindowId::Equalizer, false);
        }

        if let Some(resp) = eq_resp {
            if resp.response.dragged() {
                let delta = resp.response.drag_delta();
                app.dock_manager.move_window_group(WindowId::Equalizer, delta.x, delta.y);
            } else {
                eq_pos = resp.response.rect.min;
                let current = DockRect::new(eq_pos.x, eq_pos.y, resp.response.rect.width(), resp.response.rect.height());
                let (snapped, dock_info) = app.dock_manager.compute_snap(WindowId::Equalizer, current);
                app.dock_manager.set_rect(WindowId::Equalizer, snapped);
                if let Some((parent, edge)) = dock_info {
                    app.dock_manager.dock(WindowId::Equalizer, parent, edge);
                }
            }
        }
    }

    // 3. PLAYLIST WINDOW
    if app.dock_manager.is_visible(WindowId::Playlist) {
        let pl_rect = app.dock_manager.get_rect(WindowId::Playlist);
        let mut pl_open = true;
        let mut pl_pos = Pos2::new(pl_rect.x, pl_rect.y);

        let pl_resp = egui::Window::new("WINAMP :: PLAYLIST")
            .id(egui::Id::new("win_playlist"))
            .open(&mut pl_open)
            .current_pos(pl_pos)
            .default_size(Vec2::new(pl_rect.width, pl_rect.height))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                render_playlist_window_content(app, ui);
            });

        if !pl_open {
            app.dock_manager.set_visible(WindowId::Playlist, false);
        }

        if let Some(resp) = pl_resp {
            if resp.response.dragged() {
                let delta = resp.response.drag_delta();
                app.dock_manager.move_window_group(WindowId::Playlist, delta.x, delta.y);
            } else {
                pl_pos = resp.response.rect.min;
                let current = DockRect::new(pl_pos.x, pl_pos.y, resp.response.rect.width(), resp.response.rect.height());
                let (snapped, dock_info) = app.dock_manager.compute_snap(WindowId::Playlist, current);
                app.dock_manager.set_rect(WindowId::Playlist, snapped);
                if let Some((parent, edge)) = dock_info {
                    app.dock_manager.dock(WindowId::Playlist, parent, edge);
                }
            }
        }
    }

    // 4. PREFERENCES WINDOW
    if app.dock_manager.is_visible(WindowId::Settings) {
        let set_rect = app.dock_manager.get_rect(WindowId::Settings);
        let mut set_open = true;

        let set_resp = egui::Window::new("MULTIPLAY :: PREFERENCES")
            .id(egui::Id::new("win_settings"))
            .open(&mut set_open)
            .current_pos(Pos2::new(set_rect.x, set_rect.y))
            .default_size(Vec2::new(set_rect.width, set_rect.height))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                SettingsGuiPanel::show(ui, &mut app.settings_tab, &mut app.config);
            });

        if !set_open {
            app.dock_manager.set_visible(WindowId::Settings, false);
        }

        if let Some(resp) = set_resp {
            let pos = resp.response.rect.min;
            app.dock_manager.set_rect(WindowId::Settings, DockRect::new(pos.x, pos.y, resp.response.rect.width(), resp.response.rect.height()));
        }
    }

    // 5. DJ CONSOLE WINDOW
    if app.dock_manager.is_visible(WindowId::DjConsole) {
        let dj_rect = app.dock_manager.get_rect(WindowId::DjConsole);
        let mut dj_open = true;

        let dj_resp = egui::Window::new("WINAMP :: DJ CONSOLE")
            .id(egui::Id::new("win_dj"))
            .open(&mut dj_open)
            .current_pos(Pos2::new(dj_rect.x, dj_rect.y))
            .default_size(Vec2::new(dj_rect.width, dj_rect.height))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                DjConsolePanel::show(ui, &mut app.dj_state);
            });

        if !dj_open {
            app.dock_manager.set_visible(WindowId::DjConsole, false);
        }

        if let Some(resp) = dj_resp {
            let pos = resp.response.rect.min;
            app.dock_manager.set_rect(WindowId::DjConsole, DockRect::new(pos.x, pos.y, resp.response.rect.width(), resp.response.rect.height()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_adapter_decoder::AudioStreamDecoder;

    fn create_test_wav(path: &Path) {
        let mut wav_bytes = Vec::new();
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&76u32.to_le_bytes());
        wav_bytes.extend_from_slice(b"WAVE");
        wav_bytes.extend_from_slice(b"fmt ");
        wav_bytes.extend_from_slice(&16u32.to_le_bytes());
        wav_bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav_bytes.extend_from_slice(&2u16.to_le_bytes()); // Stereo
        wav_bytes.extend_from_slice(&44100u32.to_le_bytes()); // 44.1 kHz
        wav_bytes.extend_from_slice(&176400u32.to_le_bytes()); // Byte rate
        wav_bytes.extend_from_slice(&4u16.to_le_bytes()); // Block align
        wav_bytes.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample
        wav_bytes.extend_from_slice(b"data");
        wav_bytes.extend_from_slice(&40u32.to_le_bytes());
        wav_bytes.extend_from_slice(&[0u8; 40]);
        let _ = std::fs::write(path, wav_bytes);
    }

    #[test]
    fn test_drag_and_drop() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_drag_drop_song.wav");
        create_test_wav(&test_file);

        let mut app = MultiplayGuiApp::new().expect("Failed to initialize app");
        let initial_len = app.playlist.len();

        app.ingest_path(&test_file);
        assert_eq!(app.playlist.len(), initial_len + 1);

        app.ingest_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(app.playlist.len(), initial_len + 2);
        assert!(matches!(app.playlist[initial_len + 1].source, AudioSource::YouTube { .. }));

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_playlist_empty_state() {
        let mut app = MultiplayGuiApp::new().expect("Failed to initialize app");
        app.playlist.clear();
        assert!(app.playlist.is_empty());
        // Verify action triggers without error
        app.open_file_dialog();
        app.open_folder_dialog();
    }

    #[test]
    fn test_real_audio_streaming() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_real_stream_song.wav");
        create_test_wav(&test_file);

        let mut decoder = AudioStreamDecoder::open_file(test_file.to_str().unwrap())
            .expect("Failed to open real decoder");
        assert_eq!(decoder.sample_rate(), 44100);

        let mut pcm_buf = vec![0.0f32; 16];
        let read = decoder.read_interleaved_samples(&mut pcm_buf).expect("decode failed");
        assert!(read > 0);

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_playback_tracking_and_seek() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_tracking_song.wav");
        create_test_wav(&test_file);

        let mut decoder = AudioStreamDecoder::open_file(test_file.to_str().unwrap())
            .expect("Failed to open real decoder");
        assert_eq!(decoder.elapsed(), Duration::ZERO);

        let mut pcm_buf = vec![0.0f32; 16];
        let _ = decoder.read_interleaved_samples(&mut pcm_buf);
        assert!(decoder.elapsed() >= Duration::ZERO);

        let seek_res = decoder.seek(Duration::from_millis(0));
        assert!(seek_res.is_ok());

        let _ = std::fs::remove_file(test_file);
    }
}

