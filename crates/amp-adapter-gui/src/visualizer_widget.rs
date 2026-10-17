//! Custom painter for dual 16-band FFT audio spectrum analyzer and oscilloscope widget.

#![forbid(unsafe_code)]

use amp_core_engine::WinampTheme;
use eframe::egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

/// Visualizer display mode (classic Winamp toggle on click).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualizerMode {
    #[default]
    Spectrum,
    Oscilloscope,
}

pub struct VisualizerWidget<'a> {
    pub mode: VisualizerMode,
    pub spectrum: &'a [f32; 16],
    pub waveform: &'a [f32; 64],
    pub theme: &'a WinampTheme,
    pub height: f32,
}

impl<'a> VisualizerWidget<'a> {
    pub fn new(
        mode: VisualizerMode,
        spectrum: &'a [f32; 16],
        waveform: &'a [f32; 64],
        theme: &'a WinampTheme,
        height: f32,
    ) -> Self {
        Self {
            mode,
            spectrum,
            waveform,
            theme,
            height,
        }
    }

    pub fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), self.height), Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);

            let (bg_r, bg_g, bg_b) = self.theme.viscolor_bg;
            painter.rect_filled(rect, 4.0, Color32::from_rgb(bg_r, bg_g, bg_b));
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(1.0_f32, Color32::from_rgb(30, 42, 54)),
            );

            let padding = 4.0;
            let available_w = rect.width() - (padding * 2.0);
            let max_h = rect.height() - (padding * 2.0);

            match self.mode {
                VisualizerMode::Spectrum => {
                    self.render_spectrum(&painter, rect, padding, available_w, max_h);
                }
                VisualizerMode::Oscilloscope => {
                    self.render_oscilloscope(&painter, rect, padding, available_w, max_h);
                }
            }
        }

        response
    }

    fn render_spectrum(
        &self,
        painter: &eframe::egui::Painter,
        rect: Rect,
        padding: f32,
        available_w: f32,
        max_h: f32,
    ) {
        let num_bars = 16;
        let bar_gap = 2.0;
        let bar_w = (available_w - (bar_gap * (num_bars - 1) as f32)) / num_bars as f32;

        for (i, &mag) in self.spectrum.iter().enumerate() {
            let x = rect.left() + padding + (i as f32 * (bar_w + bar_gap));
            let clamped_mag = mag.clamp(0.02, 1.0);
            let bar_h = clamped_mag * max_h;
            let y = rect.bottom() - padding - bar_h;

            let bar_rect = Rect::from_min_max(
                Pos2::new(x, y),
                Pos2::new(x + bar_w, rect.bottom() - padding),
            );

            // Use theme viscolor gradient (16 colors indexed 0 to 15)
            let (r, g, b) = self.theme.viscolor_gradient[i];
            let color = Color32::from_rgb(r, g, b);
            painter.rect_filled(bar_rect, 1.0, color);

            // Peak indicator dot with theme peak color
            let peak_y =
                (rect.bottom() - padding - (clamped_mag * max_h) - 2.0).max(rect.top() + padding);
            let peak_rect =
                Rect::from_min_max(Pos2::new(x, peak_y), Pos2::new(x + bar_w, peak_y + 1.5));
            let (dr, dg, db) = self.theme.viscolor_dots;
            painter.rect_filled(peak_rect, 0.5, Color32::from_rgb(dr, dg, db));
        }
    }

    fn render_oscilloscope(
        &self,
        painter: &eframe::egui::Painter,
        rect: Rect,
        padding: f32,
        available_w: f32,
        max_h: f32,
    ) {
        let center_y = rect.center().y;
        let (or, og, ob) = self.theme.viscolor_osc[0];
        let glow_col = Color32::from_rgba_unmultiplied(or, og, ob, 80);
        let core_col = Color32::from_rgb(or, og, ob);

        let n = self.waveform.len().max(2);
        for j in 0..n - 1 {
            let x1 = rect.left() + padding + (j as f32 / (n - 1) as f32) * available_w;
            let y1 = center_y - (self.waveform[j].clamp(-1.0, 1.0) * (max_h * 0.45));
            let x2 = rect.left() + padding + ((j + 1) as f32 / (n - 1) as f32) * available_w;
            let y2 = center_y - (self.waveform[j + 1].clamp(-1.0, 1.0) * (max_h * 0.45));

            let p1 = Pos2::new(x1, y1);
            let p2 = Pos2::new(x2, y2);

            painter.line_segment([p1, p2], Stroke::new(2.5_f32, glow_col));
            painter.line_segment([p1, p2], Stroke::new(1.0_f32, core_col));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualizer_mode_toggle() {
        let mut mode = VisualizerMode::Spectrum;
        assert_eq!(mode, VisualizerMode::Spectrum);
        mode = VisualizerMode::Oscilloscope;
        assert_eq!(mode, VisualizerMode::Oscilloscope);
    }
}
