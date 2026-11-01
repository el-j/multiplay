//! The About tab: version, privacy statement and the third-party license bundle.

#![forbid(unsafe_code)]

use amp_core_engine::diagnostics::{self, AppInfo};
use amp_domain::about::{self, LICENSES_FILE_NAME, PRIVACY_URL, REPOSITORY_URL};
use amp_domain::AppConfig;
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

fn app_info() -> AppInfo {
    AppInfo {
        name: "multiplay",
        version_line: version_line(),
    }
}

/// Text for a bug report: version, system, latest crash, redacted settings and the log's end.
#[must_use]
pub fn diagnostics_for(config: &AppConfig, log_dir: &Path) -> String {
    diagnostics::diagnostics_text(&app_info(), &diagnostics::config_toml(config), log_dir)
}

/// Writes the diagnostics zip (summary, logs, crash reports) into the log folder.
///
/// # Errors
/// When the zip cannot be written; the message says why.
pub fn save_diagnostics(config: &AppConfig, log_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(log_dir).map_err(|e| e.to_string())?;
    diagnostics::write_diagnostics_zip(&app_info(), &diagnostics::config_toml(config), log_dir)
        .map_err(|e| e.to_string())
}

/// What the About tab asks the app to do after drawing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AboutAction {
    #[default]
    None,
    CopyDiagnostics,
    SaveDiagnostics,
    OpenLogFolder,
    /// Run the first-run setup again (the app handles this one).
    RunSetup,
}

/// Runs an About-tab action and returns the line to show under the buttons.
pub fn run_action(
    action: AboutAction,
    ctx: &eframe::egui::Context,
    config: &AppConfig,
    log_dir: &Path,
) -> Option<String> {
    match action {
        AboutAction::None | AboutAction::RunSetup => None,
        AboutAction::CopyDiagnostics => {
            ctx.copy_text(diagnostics_for(config, log_dir));
            Some("Diagnostics copied. Paste them into your issue.".into())
        }
        AboutAction::SaveDiagnostics => Some(match save_diagnostics(config, log_dir) {
            Ok(zip) => {
                open_with_system(&log_dir.to_string_lossy());
                format!("Saved {}", zip.display())
            }
            Err(e) => format!("Could not save diagnostics: {e}"),
        }),
        AboutAction::OpenLogFolder => {
            let _ = std::fs::create_dir_all(log_dir);
            open_with_system(&log_dir.to_string_lossy());
            None
        }
    }
}

fn notice_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("about-diagnostics-notice")
}

/// Shows `notice` under the diagnostics buttons until the next action.
pub fn set_notice(ctx: &eframe::egui::Context, notice: String) {
    ctx.data_mut(|d| d.insert_temp(notice_id(), notice));
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
pub fn show(ui: &mut Ui, action: &mut AboutAction) {
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

    ui.label(RichText::new("Problems?").strong());
    ui.label(
        "Logs are kept for a week. Diagnostics contain the version, your settings (passwords \
         removed) and the end of the log; nothing is sent anywhere.",
    );
    ui.horizontal_wrapped(|ui| {
        if ui.button("Copy diagnostics").clicked() {
            *action = AboutAction::CopyDiagnostics;
        }
        if ui.button("Save diagnostics (.zip)").clicked() {
            *action = AboutAction::SaveDiagnostics;
        }
        if ui.button("Open log folder").clicked() {
            *action = AboutAction::OpenLogFolder;
        }
    });
    if let Some(notice) = ui.ctx().data(|d| d.get_temp::<String>(notice_id())) {
        ui.label(RichText::new(notice).size(10.0));
    }
    ui.add_space(6.0);
    if ui.button("Run the first-run setup again").clicked() {
        *action = AboutAction::RunSetup;
    }
    if ui.button("Source code and issues").clicked() {
        open_with_system(REPOSITORY_URL);
    }
}
