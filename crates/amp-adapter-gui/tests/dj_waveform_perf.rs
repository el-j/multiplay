//! TASK-211 budget: both decks' detail lanes, the phase meter and both overviews draw in under
//! 4 ms per frame, from cached analysis only. Release builds only.

use amp_adapter_gui::dj_waveform::{overview, paint_lane, paint_phase_meter, LaneView, Marks};
use amp_domain::{TrackAnalysis, ANALYSIS_VERSION};
use eframe::egui::{self, Color32, Pos2, RawInput, Rect, Vec2};

fn analysis(bpm: f64) -> TrackAnalysis {
    TrackAnalysis {
        version: ANALYSIS_VERSION,
        duration_secs: 360.0,
        bpm: Some(bpm),
        first_downbeat_secs: 0.1,
        beats_per_bar: 4,
        overview: (0..1_024).map(|i| (i * 7 % 255) as u8).collect(),
        detail: (0..36_000)
            .map(|i| [(i % 200) as u8, (i % 150) as u8, (i % 90) as u8])
            .collect(),
        detail_rate: 100.0,
        loudness_lufs: Some(-9.0),
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "timing target applies to release builds")]
fn dj_waveform_frame_draws_under_4_ms() {
    let (a, b) = (analysis(124.0), analysis(128.0));
    let ctx = egui::Context::default();
    let frames = 120;
    let mut total = std::time::Duration::ZERO;
    for f in 0..frames {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1_000.0, 400.0))),
            ..Default::default()
        };
        let start = std::time::Instant::now();
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let painter = ui.painter().clone();
                let pos = 60.0 + f as f64 / 60.0;
                for (i, an) in [&a, &b].into_iter().enumerate() {
                    let lane = Rect::from_min_size(
                        Pos2::new(0.0, i as f32 * 60.0),
                        Vec2::new(1_000.0, 56.0),
                    );
                    let view = LaneView {
                        analysis: an,
                        position: pos,
                        speed: 1.0,
                        color: Color32::LIGHT_BLUE,
                    };
                    paint_lane(&painter, lane, &view);
                    let ov = Rect::from_min_size(
                        Pos2::new(0.0, 200.0 + i as f32 * 40.0),
                        Vec2::new(450.0, 36.0),
                    );
                    let cues = [Some(30.0), None, Some(90.0), None, None, None, None, None];
                    let marks = Marks {
                        cues: &cues,
                        loop_range: Some((40.0, 42.0)),
                        loop_active: true,
                    };
                    let _ = overview(ui, ov, Some(an), pos, 360.0, Color32::LIGHT_BLUE, marks);
                }
                paint_phase_meter(
                    &painter,
                    Rect::from_min_size(Pos2::new(350.0, 57.0), Vec2::new(300.0, 3.0)),
                    Some(0.01),
                );
            });
        });
        // Offscreen: no renderer applies texture updates, so drop them explicitly.
        output.textures_delta.clear();
        if f >= 20 {
            total += start.elapsed();
        }
    }
    let per_frame = total / (frames - 20);
    eprintln!("waveforms: {per_frame:?} per frame");
    assert!(
        per_frame < std::time::Duration::from_millis(4),
        "{per_frame:?} per frame"
    );
}
