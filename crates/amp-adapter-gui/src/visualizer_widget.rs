//! Custom painter for the 16-band FFT audio spectrum analyzer widget in egui.

#![forbid(unsafe_code)]

use eframe::egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

pub struct VisualizerWidget<'a> {
    spectrum: &'a [f32; 16],
    height: f32,
}

impl<'a> VisualizerWidget<'a> {
    pub fn new(spectrum: &'a [f32; 16], height: f32) -> Self {
        Self { spectrum, height }
    }

    pub fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), self.height),
            Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);

            // Draw dark background panel with subtle inner border
            painter.rect_filled(
                rect,
                4.0,
                Color32::from_rgb(10, 14, 18),
            );
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(1.0_f32, Color32::from_rgb(30, 42, 54)),
            );

            let num_bars = 16;
            let padding = 4.0;
            let available_w = rect.width() - (padding * 2.0);
            let bar_gap = 2.0;
            let bar_w = (available_w - (bar_gap * (num_bars - 1) as f32)) / num_bars as f32;
            let bar_max_h = rect.height() - (padding * 2.0);

            for (i, &mag) in self.spectrum.iter().enumerate() {
                let x = rect.left() + padding + (i as f32 * (bar_w + bar_gap));
                let clamped_mag = mag.clamp(0.02, 1.0);
                let bar_h = clamped_mag * bar_max_h;
                let y = rect.bottom() - padding - bar_h;

                let bar_rect = Rect::from_min_max(
                    Pos2::new(x, y),
                    Pos2::new(x + bar_w, rect.bottom() - padding),
                );

                // Multi-stop Winamp gradient: Green -> Yellow -> Orange -> Pink/Red
                let color = if clamped_mag > 0.85 {
                    Color32::from_rgb(255, 60, 110)
                } else if clamped_mag > 0.65 {
                    Color32::from_rgb(255, 175, 40)
                } else if clamped_mag > 0.35 {
                    Color32::from_rgb(0, 230, 140)
                } else {
                    Color32::from_rgb(0, 180, 220)
                };

                painter.rect_filled(bar_rect, 1.0, color);

                // Peak indicator pip
                let peak_y = (rect.bottom() - padding - (clamped_mag * bar_max_h) - 2.0).max(rect.top() + padding);
                let peak_rect = Rect::from_min_max(
                    Pos2::new(x, peak_y),
                    Pos2::new(x + bar_w, peak_y + 1.5),
                );
                painter.rect_filled(peak_rect, 0.5, Color32::from_rgb(240, 255, 255));
            }
        }

        response
    }
}
