//! Native desktop file and directory dialog integration.
//! Pure-Rust, zero C-compiler dependencies, native platform dialogs.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::Command;

/// Supported audio extensions for file picker filtering.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "ogg", "opus", "m4a", "wsz", "m3u", "m3u8",
];

/// Opens a native OS dialog to pick one or more audio files.
pub fn pick_audio_files() -> Vec<PathBuf> {
    if cfg!(test) || std::env::var("CI").is_ok() || std::env::var("HEADLESS").is_ok() {
        return Vec::new();
    }

    #[cfg(target_os = "macos")]
    {
        pick_audio_files_macos()
    }
    #[cfg(target_os = "linux")]
    {
        pick_audio_files_linux()
    }
    #[cfg(target_os = "windows")]
    {
        pick_audio_files_windows()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// Opens a native OS dialog to pick a directory.
pub fn pick_folder() -> Option<PathBuf> {
    if cfg!(test) || std::env::var("CI").is_ok() || std::env::var("HEADLESS").is_ok() {
        return None;
    }

    #[cfg(target_os = "macos")]
    {
        pick_folder_macos()
    }
    #[cfg(target_os = "linux")]
    {
        pick_folder_linux()
    }
    #[cfg(target_os = "windows")]
    {
        pick_folder_windows()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn pick_audio_files_macos() -> Vec<PathBuf> {
    let script = r#"
        set typeList to {"mp3", "flac", "wav", "aac", "ogg", "opus", "m4a", "wsz", "m3u", "m3u8"}
        try
            set selectedFiles to choose file with prompt "Select Audio Files" of type typeList with multiple selections allowed
            set posixList to {}
            repeat with aFile in selectedFiles
                set end of posixList to POSIX path of aFile
            end repeat
            set AppleScript's text item delimiters to linefeed
            return posixList as text
        on error
            return ""
        end try
    "#;

    let output = Command::new("osascript").arg("-e").arg(script).output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            return text
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect();
        }
    }
    Vec::new()
}

#[cfg(target_os = "macos")]
fn pick_folder_macos() -> Option<PathBuf> {
    let script = r#"
        try
            set selectedFolder to choose folder with prompt "Select Music Directory"
            return POSIX path of selectedFolder
        on error
            return ""
        end try
    "#;

    let output = Command::new("osascript").arg("-e").arg(script).output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn pick_audio_files_linux() -> Vec<PathBuf> {
    let filter = "*.mp3 *.flac *.wav *.aac *.ogg *.opus *.m4a *.wsz *.m3u *.m3u8";
    let output = Command::new("zenity")
        .args(["--file-selection", "--multiple", "--separator=\n"])
        .arg(format!("--file-filter=Audio Files | {filter}"))
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            return text
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect();
        }
    }
    Vec::new()
}

#[cfg(target_os = "linux")]
fn pick_folder_linux() -> Option<PathBuf> {
    let output = Command::new("zenity")
        .args(["--file-selection", "--directory"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn pick_audio_files_windows() -> Vec<PathBuf> {
    let ps_cmd = r#"
        Add-Type -AssemblyName System.Windows.Forms
        $f = New-Object System.Windows.Forms.OpenFileDialog
        $f.Filter = 'Audio Files (*.mp3;*.flac;*.wav;*.aac;*.ogg;*.m4a;*.wsz)|*.mp3;*.flac;*.wav;*.aac;*.ogg;*.m4a;*.wsz|All Files (*.*)|*.*'
        $f.Multiselect = $true
        if ($f.ShowDialog() -eq 'OK') {
            $f.FileNames | ForEach-Object { $_ }
        }
    "#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_cmd])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            return text
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect();
        }
    }
    Vec::new()
}

#[cfg(target_os = "windows")]
fn pick_folder_windows() -> Option<PathBuf> {
    let ps_cmd = r#"
        Add-Type -AssemblyName System.Windows.Forms
        $f = New-Object System.Windows.Forms.FolderBrowserDialog
        if ($f.ShowDialog() -eq 'OK') {
            $f.SelectedPath
        }
    "#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_cmd])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_dialog_supported_extensions() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"mp3"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"flac"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"wav"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"wsz"));
    }

    #[test]
    fn test_pick_files_never_opens_dialogs_under_test() {
        // `cfg!(test)` short-circuits the native dialogs; no process-global env mutation needed.
        assert!(pick_audio_files().is_empty());
        assert!(pick_folder().is_none());
    }
}
