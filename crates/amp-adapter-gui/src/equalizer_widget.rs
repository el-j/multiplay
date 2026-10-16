//! 10-Band Graphic Equalizer slider panel widget in egui.

#![forbid(unsafe_code)]

use eframe::egui::{self, Color32, Slider, Ui};

pub struct EqualizerPanel<'a> {
    gains: &'a mut [f32; 10],
    is_enabled: &'a mut bool,
}

impl<'a> EqualizerPanel<'a> {
    pub fn new(gains: &'a mut [f32; 10], is_enabled: &'a mut bool) -> Self {
        Self { gains, is_enabled }
    }

    pub fn ui(self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("10-BAND GRAPHIC EQUALIZER").size(14.0).strong().color(Color32::from_rgb(0, 230, 140)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("RESET FLAT").size(11.0)).clicked() {
                        *self.gains = [0.0; 10];
                    }

                    let btn_color = if *self.is_enabled {
                        Color32::from_rgb(0, 180, 100)
                    } else {
                        Color32::from_rgb(80, 80, 80)
                    };
                    let status_txt = if *self.is_enabled { "EQ: ACTIVE" } else { "EQ: BYPASS" };
                    if ui.button(egui::RichText::new(status_txt).size(11.0).color(btn_color)).clicked() {
                        *self.is_enabled = !*self.is_enabled;
                    }
                });
            });

            ui.add_space(4.0);

            // 10 Frequency Band Sliders
            let labels = ["60Hz", "170Hz", "310Hz", "600Hz", "1kHz", "3kHz", "6kHz", "12kHz", "14kHz", "16kHz"];
            ui.horizontal(|ui| {
                for (i, &label) in labels.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_width(34.0);
                        ui.label(egui::RichText::new(format!("{:+02.0}", self.gains[i])).size(10.0).color(Color32::from_rgb(180, 200, 220)));

                        let slider = Slider::new(&mut self.gains[i], -12.0..=12.0)
                            .vertical()
                            .show_value(false)
                            .trailing_fill(true);
                        ui.add_sized([24.0, 90.0], slider);

                        ui.label(egui::RichText::new(label).size(10.0).color(Color32::from_rgb(120, 140, 160)));
                    });
                }
            });
        });
    }
}
