//! 10-Band Graphic Equalizer with Preamp gain staging in egui.
//! Responsive: the 11 slider columns share the window width and the sliders take all spare
//! height; presets live in a dropdown so the header never overflows.

#![forbid(unsafe_code)]

use crate::responsive::row_rtl;
use eframe::egui::{self, Color32, RichText, Slider, Ui};

const LABELS: [&str; 10] = [
    "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K",
];
const PRESETS: [(&str, f32, [f32; 10]); 3] = [
    ("Flat", 0.0, [0.0; 10]),
    (
        "Bass",
        -2.0,
        [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "Rock",
        -1.0,
        [4.5, 3.0, 1.5, 0.0, -1.0, -0.5, 1.5, 3.5, 4.5, 5.0],
    ),
];
/// Value label above and frequency label below each slider, plus spacing.
const LABEL_ROWS: f32 = 30.0;

pub struct EqualizerPanel<'a> {
    pub preamp: &'a mut f32,
    pub gains: &'a mut [f32; 10],
    pub is_enabled: &'a mut bool,
}

impl<'a> EqualizerPanel<'a> {
    pub fn new(preamp: &'a mut f32, gains: &'a mut [f32; 10], is_enabled: &'a mut bool) -> Self {
        Self {
            preamp,
            gains,
            is_enabled,
        }
    }

    pub fn ui(self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;
        row_rtl(ui, |ui| {
            egui::ComboBox::from_id_salt("eq_presets")
                .selected_text("Presets")
                .width(70.0)
                .show_ui(ui, |ui| {
                    for (name, pre, gains) in PRESETS {
                        if ui.selectable_label(false, name).clicked() {
                            *self.preamp = pre;
                            *self.gains = gains;
                        }
                    }
                });
            let (txt, color) = if *self.is_enabled {
                ("ON", Color32::from_rgb(0, 200, 110))
            } else {
                ("OFF", Color32::from_rgb(130, 130, 130))
            };
            if ui
                .selectable_label(*self.is_enabled, RichText::new(txt).color(color))
                .on_hover_text("Equalizer on / bypass")
                .clicked()
            {
                *self.is_enabled = !*self.is_enabled;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new("EQUALIZER")
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(0, 230, 140)),
                    )
                    .truncate(),
                );
            });
        });

        let slider_h = (ui.available_height() - LABEL_ROWS).max(30.0);
        ui.columns(11, |cols| {
            band(
                &mut cols[0],
                self.preamp,
                "PRE",
                slider_h,
                Color32::from_rgb(255, 190, 80),
            );
            for (i, col) in cols[1..].iter_mut().enumerate() {
                band(
                    col,
                    &mut self.gains[i],
                    LABELS[i],
                    slider_h,
                    Color32::from_rgb(180, 200, 220),
                );
            }
        });
    }
}

fn band(ui: &mut Ui, value: &mut f32, label: &str, height: f32, color: Color32) {
    ui.spacing_mut().item_spacing.y = 2.0;
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(format!("{:+.0}", value))
                .size(9.0)
                .color(color),
        );
        ui.spacing_mut().slider_width = height;
        ui.add(
            Slider::new(value, -12.0..=12.0)
                .vertical()
                .show_value(false)
                .trailing_fill(true),
        )
        .on_hover_text(format!("{label} Hz: {value:+.1} dB"));
        ui.label(
            RichText::new(label)
                .size(9.0)
                .color(Color32::from_rgb(120, 140, 160)),
        );
    });
}
