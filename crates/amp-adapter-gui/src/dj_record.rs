//! Recording the DJ mix from the console (TASK-224): REC starts a FLAC recording of the master
//! into "multiplay mixes" next to the music, REC again (or leaving DJ mode) finishes it and adds
//! it to the playlist and library.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::dj_session::DjSession;
use amp_core_engine::dj::recorder::{MixMetadata, RecordFormat};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Folder recordings go into: beside the first music folder, else in the app's data folder.
#[must_use]
pub fn mixes_dir(app: &MultiplayGuiApp) -> PathBuf {
    app.config
        .library
        .music_directories
        .first()
        .map(|d| amp_adapter_storage::expand_tilde(d, None).join("multiplay mixes"))
        .unwrap_or_else(|| app.paths.data_dir.join("mixes"))
}

/// `2026-11-01 21-30` in UTC, for file names and titles.
fn stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}-{:02}",
        rem / 3_600,
        rem % 3_600 / 60
    )
}

/// Starts or finishes the recording; returns a line for the console.
pub fn toggle_recording(app: &mut MultiplayGuiApp, dj: &mut DjSession) -> String {
    if dj.control().recorder().is_recording() {
        return finish_recording(app, dj).unwrap_or_default();
    }
    let dir = mixes_dir(app);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return format!("Cannot record into {}: {e}", dir.display());
    }
    let name = format!("multiplay mix {}", stamp());
    let path = dir.join(format!("{name}.{}", RecordFormat::Flac.extension()));
    let meta = MixMetadata {
        title: name,
        artist: "multiplay DJ".into(),
    };
    match dj
        .control()
        .recorder()
        .start(&path, RecordFormat::Flac, meta)
    {
        Ok(()) => "Recording the mix".into(),
        Err(e) => format!("Cannot record: {e}"),
    }
}

/// Finishes a running recording and adds it to the playlist; `None` when nothing recorded.
pub fn finish_recording(app: &mut MultiplayGuiApp, dj: &mut DjSession) -> Option<String> {
    if !dj.control().recorder().is_recording() {
        return None;
    }
    Some(match dj.control().recorder().stop() {
        Ok(info) => {
            let secs = dj.control().recorder().seconds() as u64;
            let lost = if info.dropped_frames > 0 {
                format!(
                    " ({} frames lost: the disk was too slow)",
                    info.dropped_frames
                )
            } else {
                String::new()
            };
            app.start_import(vec![info.path.clone()]);
            format!(
                "Saved a {}:{:02} mix to {}{lost}",
                secs / 60,
                secs % 60,
                info.path.display()
            )
        }
        Err(e) => format!("The recording failed: {e}"),
    })
}

/// "REC 3:07" while recording.
#[must_use]
pub fn rec_label(dj: &mut DjSession) -> String {
    let r = dj.control().recorder();
    if r.is_recording() {
        let s = r.seconds() as u64;
        format!("REC {}:{:02}", s / 60, s % 60)
    } else {
        "REC".into()
    }
}

/// Shows `text` in the console's message line.
pub fn tell(st: &mut crate::dj_controls::ConsoleState, text: String) {
    if !text.is_empty() {
        st.message = Some((text, Instant::now()));
    }
}
