//! Every window's content must fit its window at every allowed size: nothing drawn outside,
//! nothing clipped. Renders each window offscreen with egui (no GPU, no OS window).

#![forbid(unsafe_code)]

use amp_adapter_gui::chrome::layout::TITLE_HEIGHT;
use amp_adapter_gui::chrome::resize::min_inner_size;
use amp_adapter_gui::viewports::window_body;
use amp_adapter_gui::{MultiplayGuiApp, WindowId};
use amp_domain::{AudioSource, Track, TrackMetadata};
use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};
use std::path::PathBuf;

const WINDOWS: [WindowId; 5] = [
    WindowId::Main,
    WindowId::Equalizer,
    WindowId::Playlist,
    WindowId::Settings,
    WindowId::DjConsole,
];

struct TestRoot(PathBuf);

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app_with_long_content() -> (TestRoot, MultiplayGuiApp) {
    let dir = std::env::temp_dir().join(format!("multiplay-responsive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = MultiplayGuiApp::headless(&dir).expect("headless app");
    let long =
        "An Extremely Long Track Title That Would Never Fit Into A Narrow Window (Extended Mix)";
    for i in 0..30 {
        app.playlist.push(Track::new(
            AudioSource::LocalFile {
                path: format!("/music/{i}.flac"),
            },
            TrackMetadata::new(long, "A Very Long Artist Name Featuring Another Artist"),
        ));
    }
    (TestRoot(dir), app)
}

/// Renders `window` at `size` for a few frames and returns how far content spills past the
/// window's content area (0 when it fits).
fn overflow(app: &mut MultiplayGuiApp, window: WindowId, size: Vec2) -> Vec2 {
    let ctx = egui::Context::default();
    let mut spill = Vec2::ZERO;
    for _ in 0..3 {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::TopBottomPanel::top("title")
                .exact_height(TITLE_HEIGHT)
                .show(ctx, |_| {});
            egui::CentralPanel::default()
                .frame(egui::Frame::none().inner_margin(8.0))
                .show(ctx, |ui| {
                    let avail = ui.max_rect();
                    window_body(app, window, ui);
                    let used = ui.min_rect();
                    spill = Vec2::new(
                        (used.right() - avail.right()).max(0.0),
                        (used.bottom() - avail.bottom()).max(0.0),
                    );
                });
        });
    }
    spill
}

#[test]
fn responsive_every_window_fits_at_its_minimum_size() {
    let (_root, mut app) = app_with_long_content();
    for window in WINDOWS {
        let min = min_inner_size(window);
        let spill = overflow(&mut app, window, min);
        assert!(
            spill.x <= 0.5 && spill.y <= 0.5,
            "{window:?} overflows at its minimum {min:?} by {spill:?}"
        );
    }
}

#[test]
fn responsive_every_window_fits_across_widths_and_heights() {
    let (_root, mut app) = app_with_long_content();
    for window in WINDOWS {
        let min = min_inner_size(window);
        for w in [300.0, 400.0, 600.0, 900.0] {
            for h in [min.y, min.y + 120.0, 700.0] {
                let size = Vec2::new(f32::max(w, min.x), h);
                let spill = overflow(&mut app, window, size);
                assert!(
                    spill.x <= 0.5 && spill.y <= 0.5,
                    "{window:?} overflows at {size:?} by {spill:?}"
                );
            }
        }
    }
}

#[test]
fn responsive_empty_playlist_fits_at_minimum() {
    let dir = std::env::temp_dir().join(format!("multiplay-responsive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let _root = TestRoot(dir.clone());
    let mut app = MultiplayGuiApp::headless(&dir).expect("headless app");
    let min = min_inner_size(WindowId::Playlist);
    let spill = overflow(&mut app, WindowId::Playlist, min);
    assert!(
        spill.x <= 0.5 && spill.y <= 0.5,
        "empty playlist overflows by {spill:?}"
    );
}

#[test]
fn responsive_every_settings_tab_fits() {
    use amp_adapter_gui::settings_panel::GuiSettingsTab;
    let (_root, mut app) = app_with_long_content();
    let min = min_inner_size(WindowId::Settings);
    for tab in [
        GuiSettingsTab::Folders,
        GuiSettingsTab::Audio,
        GuiSettingsTab::Accounts,
        GuiSettingsTab::P2P,
        GuiSettingsTab::Appearance,
        GuiSettingsTab::Plugins,
        GuiSettingsTab::Remote,
    ] {
        app.settings_tab = tab;
        for size in [min, Vec2::new(520.0, 420.0)] {
            let spill = overflow(&mut app, WindowId::Settings, size);
            assert!(
                spill.x <= 0.5 && spill.y <= 0.5,
                "settings tab {tab:?} overflows at {size:?} by {spill:?}"
            );
        }
    }
}

#[test]
fn responsive_dj_console_fits_from_its_minimum_up() {
    use amp_adapter_gui::dj_mode::CONSOLE_MIN;
    let (_root, mut app) = app_with_long_content();
    app.toggle_dj_mode(None);
    assert!(app.dj_active());
    let track = app.playlist[0].clone();
    if let Some(dj) = app.dj.as_mut() {
        dj.load(&app.audio_driver, 1, track);
    }
    for size in [
        CONSOLE_MIN,
        eframe::egui::Vec2::new(550.0, 232.0),
        eframe::egui::Vec2::new(700.0, 320.0),
        eframe::egui::Vec2::new(1_000.0, 500.0),
    ] {
        let spill = overflow(&mut app, WindowId::Main, size);
        assert!(
            spill.x <= 0.5 && spill.y <= 0.5,
            "DJ console overflows at {size:?} by {spill:?}"
        );
    }
}
