//! Layout helpers shared by all windows so content adapts to the window instead of overflowing.

#![forbid(unsafe_code)]

use eframe::egui::{self, Layout, Ui};

/// Standard height of a single control row.
pub const ROW: f32 = 18.0;

/// A full-width row of fixed height laid out right to left: trailing items are placed first and
/// a final left-to-right part takes (and truncates within) whatever space is left.
pub fn row_rtl<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let size = egui::vec2(ui.available_width(), ROW);
    ui.allocate_ui_with_layout(size, Layout::right_to_left(egui::Align::Center), add)
        .inner
}
