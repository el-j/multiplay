//! UI rendering logic and root viewport window coordinator for Multiplay GUI.
//! Handles 60 FPS conditional repaint, drag-and-drop, and delegates satellite viewports.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::docking::{DockRect, WindowId};
use crate::main_window::render_main_window_content;
use crate::viewports::render_satellite_windows;
use eframe::egui::{self, Color32, Vec2};
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

        // 3. Track Root Window (Main Player) Desktop Position & Propagate Group Movement
        if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
            let old_main = self.dock_manager.get_rect(WindowId::Main);
            let dx = outer.min.x - old_main.x;
            let dy = outer.min.y - old_main.y;
            let inner_size = ctx
                .input(|i| i.viewport().inner_rect)
                .map(|r| r.size())
                .unwrap_or(Vec2::new(old_main.width, old_main.height));
            self.dock_manager.set_rect(
                WindowId::Main,
                DockRect::new(outer.min.x, outer.min.y, inner_size.x, inner_size.y),
            );
            let pointer_down = ctx.input(|i| i.pointer.any_down());
            if (dx.abs() > 1.0 || dy.abs() > 1.0) && !pointer_down {
                self.dock_manager.move_window_group(WindowId::Main, dx, dy);
                self.sync_child_viewport_positions(ctx);
            }
        }

        // 4. Render Main Player Content in Root Viewport
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(14, 18, 22))
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                render_main_window_content(self, ui);
            });

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
                amp_core_engine::PlayerCommand::SeekRelative(_, forward) => {
                    if forward {
                        if self.selected_track_idx + 1 < self.playlist.len() {
                            self.selected_track_idx += 1;
                            self.audio_driver
                                .play(self.playlist[self.selected_track_idx].clone());
                        }
                    } else if self.selected_track_idx > 0 {
                        self.selected_track_idx -= 1;
                        self.audio_driver
                            .play(self.playlist[self.selected_track_idx].clone());
                    }
                }
                amp_core_engine::PlayerCommand::SeekAbsolute(target) => {
                    self.audio_driver.seek(target);
                }
                amp_core_engine::PlayerCommand::AddToQueue(track) => {
                    self.playlist.push(*track);
                }
                _ => {}
            }
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
