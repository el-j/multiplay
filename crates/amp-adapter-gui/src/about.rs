//! The About tab: version, privacy statement and the third-party license bundle.

#![forbid(unsafe_code)]

use amp_domain::about::{self, LICENSES_FILE_NAME, PRIVACY_URL, REPOSITORY_URL};
use eframe::egui::{Color32, RichText, Ui};
use std::path::{Path, PathBuf};

/// `multiplay 0.1.0-beta.2 (1a2b3c4)`, as built.
#[must_use]
pub fn version_line() -> String {
    about::version_line(
        "multiplay",
        env!("MULTIPLAY_VERSION"),
        env!("MULTIPLAY_COMMIT"),
    )
}

/// The license bundle shipped with the download: next to the executable (Linux, Windows) or in
/// the app bundle's `Resources` folder (macOS). `None` in a development build.
#[must_use]
pub fn licenses_file_near(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    let beside = dir.join(LICENSES_FILE_NAME);
    let resources = dir
        .parent()
        .map(|c| c.join("Resources").join(LICENSES_FILE_NAME));
    std::iter::once(beside)
        .chain(resources)
        .find(|p| p.is_file())
}

/// Opens a file or URL with the system's default app.
fn open_with_system(target: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(target).spawn();
}

/// Draws the About tab.
pub fn show(ui: &mut Ui) {
    ui.label(
        RichText::new(version_line())
            .color(Color32::from_rgb(0, 235, 136))
            .strong(),
    );
    ui.label("Beta software: expect rough edges, and please report what breaks.");
    ui.add_space(6.0);

    ui.label(RichText::new("Privacy").strong());
    ui.label(
        "No telemetry, no accounts, no update checks. multiplay only goes online when you \
         search YouTube or Bandcamp, play a stream or link, or turn on remote control.",
    );
    if ui.button("Read the privacy statement").clicked() {
        open_with_system(PRIVACY_URL);
    }
    ui.add_space(6.0);

    ui.label(RichText::new("Licenses").strong());
    ui.label("multiplay is built on open-source Rust crates; their licenses ship with the app.");
    match std::env::current_exe()
        .ok()
        .and_then(|exe| licenses_file_near(&exe))
    {
        Some(file) => {
            if ui.button("Open third-party licenses").clicked() {
                open_with_system(&file.to_string_lossy());
            }
        }
        None => {
            ui.label(
                RichText::new(format!(
                    "{LICENSES_FILE_NAME} is included in release downloads."
                ))
                .size(10.0),
            );
        }
    }
    ui.add_space(6.0);
    if ui.button("Source code and issues").clicked() {
        open_with_system(REPOSITORY_URL);
    }
}
