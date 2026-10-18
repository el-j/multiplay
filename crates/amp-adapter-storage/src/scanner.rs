//! Asynchronous local audio library crawler with pure-Rust Lofty metadata tag extraction.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackId, TrackMetadata};
use amp_ports::driven::LibraryStoragePort;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct LibraryScanner;

impl LibraryScanner {
    /// Scans a directory recursively and indexes all discovered audio tracks into storage.
    pub async fn scan_directory<P: AsRef<Path>, S: LibraryStoragePort + 'static>(
        dir_path: P,
        storage: Arc<S>,
    ) -> Result<usize, String> {
        let root = dir_path.as_ref().to_path_buf();
        let files = tokio::task::spawn_blocking(move || collect_audio_files(&root))
            .await
            .map_err(|e| format!("Directory crawler panic: {e}"))?;

        let mut indexed_count = 0;

        for path in files {
            if let Some(track) = extract_track_from_file(&path) {
                if storage.store_track(&track).await.is_ok() {
                    indexed_count += 1;
                }
            }
        }

        Ok(indexed_count)
    }
}

pub fn collect_audio_files(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let supported_extensions = ["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus"];

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(collect_audio_files(&path));
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if supported_extensions.contains(&ext.to_lowercase().as_str()) {
                    results.push(path);
                }
            }
        }
    }

    results
}

pub fn extract_track_from_file(path: &Path) -> Option<Track> {
    let tagged_file = Probe::open(path).ok()?.read().ok()?;

    let properties = tagged_file.properties();
    let duration = Some(properties.duration());
    let sample_rate = properties.sample_rate();
    let channels = properties.channels().map(|c| c as u16);
    let bitrate_kbps = properties.audio_bitrate();

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let title = tag
        .and_then(|t| t.title().as_deref().map(str::to_string))
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Title")
                .to_string()
        });

    let artist = tag
        .and_then(|t| t.artist().as_deref().map(str::to_string))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = tag.and_then(|t| t.album().as_deref().map(str::to_string));
    let track_number = tag.and_then(|t| t.track());

    let (replay_gain_mb, integrated_lufs_mb) = if let Some(t) = tag {
        let rg = t
            .get(&lofty::tag::ItemKey::ReplayGainTrackGain)
            .or_else(|| {
                t.get(&lofty::tag::ItemKey::Unknown(
                    "REPLAYGAIN_TRACK_GAIN".to_string(),
                ))
            })
            .and_then(|item| item.value().text())
            .and_then(|s| s.replace("dB", "").trim().parse::<f32>().ok())
            .map(|db| (db * 100.0).round() as i32);

        let lufs = t
            .get(&lofty::tag::ItemKey::Unknown("R128_TRACK_GAIN".to_string()))
            .and_then(|item| item.value().text())
            .and_then(|s| {
                s.replace("LUFS", "")
                    .replace("dB", "")
                    .trim()
                    .parse::<f32>()
                    .ok()
            })
            .map(|v| (v * 100.0).round() as i32);

        (rg, lufs)
    } else {
        (None, None)
    };

    let metadata = TrackMetadata {
        title,
        artist,
        album,
        track_number,
        duration,
        sample_rate,
        channels,
        bit_depth: properties.bit_depth().map(|b| b as u16),
        bitrate_kbps,
        cover_art_uri: None,
        replay_gain_mb,
        integrated_lufs_mb,
    };

    Some(Track {
        id: TrackId::new(),
        source: AudioSource::LocalFile {
            path: path.to_string_lossy().to_string(),
        },
        metadata,
        date_added_epoch: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_collect_audio_files() {
        let dir = tempdir().unwrap();
        let audio1 = dir.path().join("song1.mp3");
        let audio2 = dir.path().join("song2.flac");
        let txt = dir.path().join("notes.txt");

        fs::write(&audio1, b"mock").unwrap();
        fs::write(&audio2, b"mock").unwrap();
        fs::write(&txt, b"mock").unwrap();

        let collected = collect_audio_files(dir.path());
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_probe_metadata() {
        let dir = tempdir().unwrap();
        let wav_path = dir.path().join("Artist - Title.wav");

        // Write a minimal valid WAV container
        let mut wav_bytes = Vec::new();
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&76u32.to_le_bytes());
        wav_bytes.extend_from_slice(b"WAVE");
        wav_bytes.extend_from_slice(b"fmt ");
        wav_bytes.extend_from_slice(&16u32.to_le_bytes());
        wav_bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav_bytes.extend_from_slice(&2u16.to_le_bytes()); // Stereo
        wav_bytes.extend_from_slice(&44100u32.to_le_bytes()); // 44.1 kHz
        wav_bytes.extend_from_slice(&176400u32.to_le_bytes()); // Byte rate
        wav_bytes.extend_from_slice(&4u16.to_le_bytes()); // Block align
        wav_bytes.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample
        wav_bytes.extend_from_slice(b"data");
        wav_bytes.extend_from_slice(&40u32.to_le_bytes());
        wav_bytes.extend_from_slice(&[0u8; 40]);

        fs::write(&wav_path, &wav_bytes).unwrap();

        let track = extract_track_from_file(&wav_path);
        assert!(track.is_some());
        let t = track.unwrap();
        assert_eq!(t.metadata.title, "Artist - Title");
        assert_eq!(t.metadata.sample_rate, Some(44100));
        assert_eq!(t.metadata.channels, Some(2));
    }
}
