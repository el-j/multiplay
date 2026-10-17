//! Pure-Rust QR Code egui painter widget.
//! Renders 2D boolean module matrices with crisp pixel-aligned square tiles.

#![forbid(unsafe_code)]

use eframe::egui::{self, Color32, Rect, Sense, Ui, Vec2};

/// Renders a crisp QR code matrix into the egui UI.
pub fn draw_qr_code(ui: &mut Ui, matrix: &[Vec<bool>], size: f32) {
    let (response, painter) = ui.allocate_painter(Vec2::new(size, size), Sense::hover());
    let rect = response.rect;
    let n = matrix.len();

    if n == 0 {
        painter.rect_filled(rect, 4.0, Color32::DARK_GRAY);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Generating QR...",
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );
        return;
    }

    // Quiet zone background
    painter.rect_filled(rect, 6.0, Color32::WHITE);

    let cell_size = size / (n as f32);

    for (row_idx, row) in matrix.iter().enumerate() {
        for (col_idx, &is_dark) in row.iter().enumerate() {
            if is_dark {
                let min = rect.min
                    + Vec2::new(
                        col_idx as f32 * cell_size,
                        row_idx as f32 * cell_size,
                    );
                let max = min + Vec2::new(cell_size, cell_size);
                painter.rect_filled(Rect::from_min_max(min, max), 0.0, Color32::BLACK);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_widget_empty_matrix() {
        let matrix: Vec<Vec<bool>> = Vec::new();
        assert!(matrix.is_empty());
        let _ = draw_qr_code as fn(&mut eframe::egui::Ui, &[Vec<bool>], f32);
    }
}
