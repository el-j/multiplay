//! UI rendering logic and root viewport window coordinator for Multiplay GUI.
//! Handles 60 FPS conditional repaint, drag-and-drop, and delegates satellite viewports.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::chrome;
use crate::docking::WindowId;
use crate::main_window::render_main_window_content;
use crate::viewports::render_satellite_windows;
use eframe::egui::{self, Color32};
use std::sync::atomic::Ordering;
use std::time::Duration;

impl eframe::App for MultiplayGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Smooth continuous 60 FPS repaint only while audio is playing (idle state is event-driven)
        if self.audio_driver.is_playing.load(Ordering::Relaxed) {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        // 1. Automatic Gapless Track Advancement & Audio Parameters Sync
        self.check_gapless_advancement();
        self.audio_driver
            .set_crossfade_seconds(self.config.audio.crossfade_duration_ms as f32 / 1000.0);

        // 2. OS-Native Drag-and-Drop Ingestion (Files & Folders on Main Player)
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped_files {
            if let Some(path) = f.path {
                self.ingest_path(&path);
            }
        }

        // 3. Main window: self-drawn title strip; dragging it moves the docked group live
        chrome::track_geometry(self, ctx, WindowId::Main);
        let action = chrome::title_bar(self, ctx, WindowId::Main, "multiplay", true, true);
        chrome::apply_action(self, ctx, WindowId::Main, action);

        // 4. Render Main Player Content in Root Viewport (hidden while shaded)
        if !self.dock_manager.is_shaded(WindowId::Main) {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(14, 18, 22))
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| {
                    render_main_window_content(self, ui);
                });
        }

        // 5. Render Modular Satellite OS Native Windows
        render_satellite_windows(self, ctx);

        // 6. Process Inbound Mobile Remote Commands
        self.handle_remote_commands();

        // 7. Broadcast Real-Time Player State to Mobile Remote Clients
        self.broadcast_remote_state();
    }
}

impl MultiplayGuiApp {
    fn handle_remote_commands(&mut self) {
        while let Some(cmd) = self.remote.try_recv_command() {
            match cmd {
                amp_core_engine::PlayerCommand::TogglePause => {
                    self.audio_driver.toggle_play_pause();
                }
                amp_core_engine::PlayerCommand::SetVolume(v) => {
                    self.volume = v.clamp(0.0, 1.0);
                    self.audio_driver.set_volume(self.volume);
                }
                amp_core_engine::PlayerCommand::Next => self.remote_skip(true),
                amp_core_engine::PlayerCommand::Previous => self.remote_skip(false),
                amp_core_engine::PlayerCommand::SeekAbsolute(target) => {
                    self.audio_driver.seek(target);
                }
                amp_core_engine::PlayerCommand::EnqueueExisting(id) => {
                    // Remotes may only re-queue tracks the host already has.
                    if let Some(track) = self.playlist.iter().find(|t| t.id == id).cloned() {
                        self.playlist.push(track);
                    }
                }
                _ => {}
            }
        }
    }

    fn remote_skip(&mut self, forward: bool) {
        let target = if forward {
            (self.selected_track_idx + 1 < self.playlist.len()).then(|| self.selected_track_idx + 1)
        } else {
            self.selected_track_idx.checked_sub(1)
        };
        if let Some(idx) = target {
            self.selected_track_idx = idx;
            self.audio_driver.play(self.playlist[idx].clone());
        }
    }

    fn broadcast_remote_state(&self) {
        if self.remote.active_peers_count() > 0 {
            let track = self.audio_driver.current_track.lock().unwrap().clone();
            let is_playing = self.audio_driver.is_playing.load(Ordering::Relaxed);
            let vol = *self.audio_driver.volume.lock().unwrap();
            let pos = self.audio_driver.elapsed.lock().unwrap().as_secs();
            self.remote.broadcast_status(track, is_playing, vol, pos);
        }
    }
}

/// Backwards-compatible alias for modular window rendering.
#[inline]
pub fn render_modular_windows(app: &mut MultiplayGuiApp, ctx: &egui::Context) {
    render_satellite_windows(app, ctx);
}
