//! First-run setup (TASK-171): music folders, output device, online sources with what each one
//! sends, and remote control (off). Shown once in the Preferences overlay; Preferences → About can
//! run it again.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use crate::docking::WindowId;
use amp_domain::AppConfig;
use eframe::egui::{self, Color32, RichText, Ui};
use std::path::PathBuf;

/// The setup's pages, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Music,
    Output,
    Sources,
    Remote,
    Finish,
}

const STEPS: [Step; 5] = [
    Step::Music,
    Step::Output,
    Step::Sources,
    Step::Remote,
    Step::Finish,
];

/// Choices made so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Onboarding {
    pub step: Step,
    pub folders: Vec<PathBuf>,
    /// `None`: the system default output.
    pub device: Option<String>,
    pub youtube: bool,
    pub bandcamp: bool,
    pub remote: bool,
}

impl Onboarding {
    /// Starts from the current settings; `music` (normally ~/Music) is suggested when it exists.
    #[must_use]
    pub fn new(config: &AppConfig, music: Option<PathBuf>) -> Self {
        let mut folders: Vec<PathBuf> = config
            .library
            .music_directories
            .iter()
            .map(|d| amp_adapter_storage::expand_tilde(d, None))
            .collect();
        if folders.is_empty() {
            folders.extend(music);
        }
        Self {
            step: Step::Music,
            folders,
            device: config.audio.output_device.clone(),
            youtube: config.streaming.enable_youtube,
            bandcamp: config.streaming.enable_bandcamp,
            remote: config.remote.enabled,
        }
    }

    fn position(&self) -> usize {
        STEPS.iter().position(|s| *s == self.step).unwrap_or(0)
    }

    pub fn next(&mut self) {
        self.step = STEPS[(self.position() + 1).min(STEPS.len() - 1)];
    }

    pub fn back(&mut self) {
        self.step = STEPS[self.position().saturating_sub(1)];
    }

    /// Writes the choices into `config` and marks the setup done.
    pub fn apply(&self, config: &mut AppConfig) {
        config.library.music_directories = self
            .folders
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        config.audio.output_device = self.device.clone();
        config.streaming.enable_youtube = self.youtube;
        config.streaming.enable_bandcamp = self.bandcamp;
        config.remote.enabled = self.remote;
        config.ui.first_run_done = true;
    }
}

impl MultiplayGuiApp {
    /// Opens the setup in the Preferences overlay.
    pub fn start_onboarding(&mut self) {
        self.onboarding = Some(Onboarding::new(
            &self.config,
            self.paths.music_dir.clone().filter(|p| p.is_dir()),
        ));
        self.dock_manager.set_visible(WindowId::Settings, true);
    }

    /// Saves the choices, applies them and starts scanning the chosen folders.
    pub fn finish_onboarding(&mut self) {
        let Some(ob) = self.onboarding.take() else {
            return;
        };
        ob.apply(&mut self.config);
        self.save_and_apply_config();
        self.remote.apply_config(&self.config.remote);
        self.dock_manager.set_visible(WindowId::Settings, false);
        self.start_import(ob.folders);
    }

    /// Closes the setup without changing anything; it is not shown again.
    pub fn skip_onboarding(&mut self) {
        self.onboarding = None;
        self.config.ui.first_run_done = true;
        self.save_and_apply_config();
        self.dock_manager.set_visible(WindowId::Settings, false);
    }
}

fn note(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(10.5)
            .color(Color32::from_rgb(170, 185, 200)),
    );
}

/// Draws the setup page with Back / Next / Start and Skip.
pub fn render(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let Some(ob) = app.onboarding.as_mut() else {
        return;
    };
    let accent = Color32::from_rgb(0, 235, 136);
    ui.label(
        RichText::new(format!(
            "WELCOME TO MULTIPLAY · {}/{}",
            ob.position() + 1,
            STEPS.len()
        ))
        .color(accent)
        .strong(),
    );
    ui.add_space(6.0);
    let mut pick_folder = false;
    match ob.step {
        Step::Music => {
            ui.label(RichText::new("Where is your music?").strong());
            note(ui, "These folders are scanned now and their tracks added to the playlist and the library.");
            let mut remove = None;
            for (i, f) in ob.folders.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui
                        .small_button("✖")
                        .on_hover_text("Don't scan this folder")
                        .clicked()
                    {
                        remove = Some(i);
                    }
                    ui.add(egui::Label::new(f.display().to_string()).truncate());
                });
            }
            if let Some(i) = remove {
                ob.folders.remove(i);
            }
            pick_folder = ui.button("+ Add a folder").clicked();
        }
        Step::Output => {
            ui.label(RichText::new("Which speakers or headphones?").strong());
            let names: Vec<String> = app
                .audio_driver
                .available_devices
                .iter()
                .map(|d| d.name.clone())
                .collect();
            let shown = ob.device.clone().unwrap_or_else(|| "System default".into());
            egui::ComboBox::from_id_salt("onboarding-device")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut ob.device, None, "System default");
                    for n in names {
                        ui.selectable_value(&mut ob.device, Some(n.clone()), n);
                    }
                });
            note(
                ui,
                "You can change this any time in the Audio Output tab of Preferences.",
            );
        }
        Step::Sources => {
            ui.label(RichText::new("Search online too?").strong());
            ui.checkbox(&mut ob.youtube, "YouTube");
            note(ui, "Your search text goes to youtube.com; playing a result downloads its audio from Google's servers.");
            ui.checkbox(&mut ob.bandcamp, "Bandcamp");
            note(ui, "Your search text goes to bandcamp.com; playing a result streams it from Bandcamp's servers.");
            note(ui, "With both off, nothing is sent anywhere. Your own music is always searched on this computer only.");
        }
        Step::Remote => {
            ui.label(RichText::new("Control from your phone?").strong());
            ui.checkbox(
                &mut ob.remote,
                "Turn on remote control (this computer only until you allow your network)",
            );
            note(
                ui,
                "Off by default: while it is off, multiplay listens on no network port.",
            );
        }
        Step::Finish => {
            ui.label(RichText::new("All set").strong());
            note(ui, &format!("{} folder(s) will be scanned in the background. Press space to play, ? in the terminal version for keys.", ob.folders.len()));
        }
    }
    ui.add_space(8.0);
    let (mut finish, mut skip) = (false, false);
    ui.horizontal(|ui| {
        if ob.step != Step::Music && ui.button("Back").clicked() {
            ob.back();
        }
        if ob.step == Step::Finish {
            finish = ui.button(RichText::new("Start").strong()).clicked();
        } else if ui.button(RichText::new("Next").strong()).clicked() {
            ob.next();
        }
        skip = ui.button("Skip setup").clicked();
    });
    if pick_folder {
        app.start_dialog_for_onboarding();
    }
    if finish {
        app.finish_onboarding();
    } else if skip {
        app.skip_onboarding();
    }
}
