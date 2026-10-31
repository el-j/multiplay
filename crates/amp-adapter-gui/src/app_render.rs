//! UI rendering logic and root viewport window coordinator for Multiplay GUI.
//! Handles 60 FPS conditional repaint, drag-and-drop, and delegates satellite viewports.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::chrome;
use crate::docking::WindowId;
use crate::viewports::render_satellite_windows;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

impl eframe::App for MultiplayGuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        // Smooth continuous 60 FPS repaint only while audio is playing (idle state is event-driven)
        if self.audio_driver.is_playing.load(Ordering::Relaxed) {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        // 1. Automatic Gapless Track Advancement & Audio Parameters Sync
        self.check_gapless_advancement();
        self.poll_playback_events();
        self.audio_driver
            .set_crossfade_seconds(self.config.audio.crossfade_duration_ms as f32 / 1000.0);

        self.sync_dj_headphones();

        // Double size / zoom: re-express the layout before any window geometry is read.
        chrome::zoom::sync(self, ctx);

        // 3. Main window: self-drawn title strip; dragging it moves the docked group live
        if self.initialized_viewports.insert(WindowId::Main) {
            // First frame: apply the saved zoom, bring windows from unplugged monitors back,
            // and put the root window where the layout says, at least at its minimum size.
            if (self.startup_zoom - 1.0).abs() > 1e-3 {
                ctx.set_zoom_factor(self.startup_zoom);
            }
            let all: Vec<_> = self
                .dock_manager
                .windows
                .iter()
                .map(|(id, r)| (*id, *r))
                .collect();
            for (id, r) in crate::layout_manager::fit_on_screen(&all, &chrome::screens(ctx)) {
                self.dock_manager.set_rect(id, r);
            }
            self.geometry_settle = chrome::zoom::SETTLE_FRAMES;
            let min = self.min_size(WindowId::Main);
            let mut r = self.dock_manager.get_rect(WindowId::Main);
            r.width = r.width.max(min.x);
            r.height = r.height.max(min.y);
            self.dock_manager.set_rect(WindowId::Main, r);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                r.width, r.height,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(r.x, r.y)));
        } else {
            chrome::track_geometry(self, ctx, WindowId::Main);
        }
        let action = chrome::title_bar(self, ui, WindowId::Main, "multiplay", true, true);
        chrome::apply_action(self, ctx, WindowId::Main, action);

        // 4. Render Main Player Content in Root Viewport (hidden while shaded); like every
        //    window, it also adds files and folders dropped onto it.
        crate::viewports::show_body(self, ui, WindowId::Main);

        // 5. Render Modular Satellite OS Native Windows
        render_satellite_windows(self, ctx);

        // 6. Process Inbound Mobile Remote Commands
        self.handle_remote_commands();
        self.handle_remote_requests();

        // 7. Broadcast Real-Time Player State to Mobile Remote Clients
        self.broadcast_remote_state();

        // 8. Persist the window layout once an interaction has finished
        self.save_layout_if_changed();
    }

    fn on_exit(&mut self) {
        self.save_layout();
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
