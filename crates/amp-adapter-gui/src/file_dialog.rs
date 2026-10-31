//! Native desktop file and directory dialog integration.
//! Pure-Rust, zero C-compiler dependencies, native platform dialogs.
//!
//! Both dialogs allow selecting several items. On macOS the "add" dialog takes files and folders
//! in one go (NSOpenPanel through JavaScript for Automation, no permission prompts).

#![forbid(unsafe_code)]

use std::path::PathBuf;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
use std::process::Command;

/// Files the add dialog accepts besides audio: Winamp skins.
const SKIN_EXTENSIONS: [&str; 1] = ["wsz"];

/// Extensions offered by the add dialog: every playable audio format from
/// `amp_domain::formats`, plus playlists, CUE sheets and skins.
pub fn dialog_extensions() -> Vec<&'static str> {
    amp_domain::formats::playable_extensions()
        .chain(crate::playlist_import::LIST_EXTENSIONS)
        .chain(SKIN_EXTENSIONS)
        .collect()
}

fn dialogs_disabled() -> bool {
    cfg!(test) || std::env::var("CI").is_ok() || std::env::var("HEADLESS").is_ok()
}

/// Opens a native OS dialog to pick audio files (and, on macOS, folders), several at once.
pub fn pick_audio_files() -> Vec<PathBuf> {
    if dialogs_disabled() {
        return Vec::new();
    }

    #[cfg(target_os = "macos")]
    {
        run_paths(
            Command::new("osascript")
                .args(["-l", "JavaScript", "-e"])
                .arg(macos_open_panel_script(&dialog_extensions())),
        )
    }
    #[cfg(target_os = "linux")]
    {
        let filter: Vec<String> = dialog_extensions()
            .iter()
            .map(|e| format!("*.{e}"))
            .collect();
        run_paths(
            Command::new("zenity")
                .args(["--file-selection", "--multiple", "--separator=\n"])
                .arg(format!("--file-filter=Audio Files | {}", filter.join(" "))),
        )
    }
    #[cfg(target_os = "windows")]
    {
        let patterns: Vec<String> = dialog_extensions()
            .iter()
            .map(|e| format!("*.{e}"))
            .collect();
        let patterns = patterns.join(";");
        let ps_cmd = format!(
            r#"
            Add-Type -AssemblyName System.Windows.Forms
            $f = New-Object System.Windows.Forms.OpenFileDialog
            $f.Filter = 'Audio Files ({patterns})|{patterns}|All Files (*.*)|*.*'
            $f.Multiselect = $true
            if ($f.ShowDialog() -eq 'OK') {{
                $f.FileNames | ForEach-Object {{ $_ }}
            }}
        "#
        );
        run_paths(Command::new("powershell").args(["-NoProfile", "-Command", &ps_cmd]))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// Opens a native OS dialog to pick music folders (several on macOS and Linux).
pub fn pick_folders() -> Vec<PathBuf> {
    if dialogs_disabled() {
        return Vec::new();
    }

    #[cfg(target_os = "macos")]
    {
        let script = r#"
            try
                set chosen to choose folder with prompt "Select Music Folders" with multiple selections allowed
                set out to {}
                repeat with f in chosen
                    set end of out to POSIX path of f
                end repeat
                set AppleScript's text item delimiters to linefeed
                return out as text
            on error
                return ""
            end try
        "#;
        run_paths(Command::new("osascript").arg("-e").arg(script))
    }
    #[cfg(target_os = "linux")]
    {
        run_paths(Command::new("zenity").args([
            "--file-selection",
            "--directory",
            "--multiple",
            "--separator=\n",
        ]))
    }
    #[cfg(target_os = "windows")]
    {
        // The classic folder browser picks one folder at a time.
        let ps_cmd = r#"
            Add-Type -AssemblyName System.Windows.Forms
            $f = New-Object System.Windows.Forms.FolderBrowserDialog
            if ($f.ShowDialog() -eq 'OK') {
                $f.SelectedPath
            }
        "#;
        run_paths(Command::new("powershell").args(["-NoProfile", "-Command", ps_cmd]))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// JavaScript for Automation script showing an NSOpenPanel that accepts several files of
/// `extensions` and folders at once; prints one POSIX path per line, nothing when cancelled.
pub fn macos_open_panel_script(extensions: &[&str]) -> String {
    let types: Vec<String> = extensions.iter().map(|e| format!("'{e}'")).collect();
    format!(
        r#"
        ObjC.import('AppKit');
        const app = $.NSApplication.sharedApplication;
        app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);
        app.activateIgnoringOtherApps(true);
        const panel = $.NSOpenPanel.openPanel;
        panel.title = 'Add Music';
        panel.prompt = 'Add';
        panel.message = 'Select files and folders to add (hold Cmd or Shift to select several).';
        panel.canChooseFiles = true;
        panel.canChooseDirectories = true;
        panel.allowsMultipleSelection = true;
        panel.allowedFileTypes = $([{types}]);
        let out = '';
        if (panel.runModal == $.NSModalResponseOK) {{
            const urls = panel.URLs;
            for (let i = 0; i < urls.count; i++) {{
                out += urls.objectAtIndex(i).path.js + '\n';
            }}
        }}
        out;
        "#,
        types = types.join(", ")
    )
}

/// One path per non-empty output line.
pub fn parse_paths(stdout: &str) -> Vec<PathBuf> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
fn run_paths(cmd: &mut Command) -> Vec<PathBuf> {
    match cmd.output() {
        Ok(out) if out.status.success() => parse_paths(&String::from_utf8_lossy(&out.stdout)),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_dialog_supported_extensions() {
        let exts = dialog_extensions();
        for ext in ["mp3", "flac", "wav", "wsz", "m3u8"] {
            assert!(exts.contains(&ext), "{ext}");
        }
    }

    #[test]
    fn test_pick_files_never_opens_dialogs_under_test() {
        // `cfg!(test)` short-circuits the native dialogs; no process-global env mutation needed.
        assert!(pick_audio_files().is_empty());
        assert!(pick_folders().is_empty());
    }
}
