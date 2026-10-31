//! Offline stream downloader pipeline with pure-Rust Lofty metadata tagging.

#![forbid(unsafe_code)]

use crate::events::PlayerEvent;
use amp_domain::{Track, TrackId};
use amp_ports::driven::StreamResolverPort;
use amp_ports::driving::TrackDownloadPort;
use amp_ports::error::DownloadError;
use async_trait::async_trait;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::tag::{Accessor, Tag, TagExt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, Mutex, RwLock};

/// Sanitizes a string for use in cross-platform filesystem paths.
/// Strips reserved characters on Windows/Unix (`/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`),
/// removes path traversal (`..`), protects Windows reserved devices, and bounds length.
pub fn sanitize_filename(raw: &str) -> String {
    let mut clean: String = raw
        .chars()
        .map(|c| match c {
            '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            c if (c as u32) < 32 || (c as u32) == 127 => '_',
            c => c,
        })
        .collect();

    while clean.contains("..") {
        clean = clean.replace("..", "_");
    }

    let trimmed = clean.trim_matches(|c: char| c.is_whitespace() || c == '.' || c == '_');
    let mut base = if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed.to_string()
    };

    let upper = base.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&upper.as_str()) {
        base = format!("_{base}");
    }
    if base.len() > 100 {
        base.truncate(100);
        base = base.trim_end_matches(['.', '_', ' ']).to_string();
    }
    base
}

/// Detects audio container format from header bytes.
pub fn detect_audio_container(header: &[u8]) -> &'static str {
    if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" {
        return "wav";
    }
    if header.len() >= 4 && &header[0..4] == b"fLaC" {
        return "flac";
    }
    if header.len() >= 4 && &header[0..4] == b"OggS" {
        return "ogg";
    }
    if (header.len() >= 8 && &header[4..8] == b"ftyp")
        || (header.len() >= 12 && &header[8..12] == b"ftyp")
    {
        return "m4a";
    }
    if header.len() >= 3 && &header[0..3] == b"ID3" {
        return "mp3";
    }
    if header.len() >= 2 && header[0] == 0xFF && (header[1] & 0xE0) == 0xE0 {
        return "mp3";
    }

    if let Some(file_type) = lofty::file::FileType::from_buffer(header) {
        match file_type {
            lofty::file::FileType::Mpeg => return "mp3",
            lofty::file::FileType::Flac => return "flac",
            lofty::file::FileType::Wav => return "wav",
            lofty::file::FileType::Mp4 | lofty::file::FileType::Aac => return "m4a",
            lofty::file::FileType::Vorbis | lofty::file::FileType::Opus => return "ogg",
            _ => {}
        }
    }
    "mp3"
}

/// Computes a non-colliding destination path by suffixing ` (2)`, ` (3)`, etc.
pub fn get_unique_filepath(dest_dir: &Path, base_name: &str, ext: &str) -> PathBuf {
    let candidate = dest_dir.join(format!("{base_name}.{ext}"));
    if !candidate.exists() {
        return candidate;
    }
    let mut index = 2;
    loop {
        let candidate = dest_dir.join(format!("{base_name} ({index}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

pub struct OfflineTrackDownloader;

impl OfflineTrackDownloader {
    /// Downloads an audio stream to destination disk folder and embeds Lofty metadata tags.
    pub async fn download_and_tag(
        resolver: &dyn StreamResolverPort,
        track: &Track,
        destination_folder: &Path,
        cancel_flag: Option<Arc<AtomicBool>>,
        event_tx: Option<&broadcast::Sender<PlayerEvent>>,
    ) -> Result<PathBuf, DownloadError> {
        tokio::fs::create_dir_all(destination_folder)
            .await
            .map_err(|e| DownloadError::Io(format!("Failed to create download folder: {e}")))?;

        let mut source_stream = resolver
            .resolve_stream(&track.source)
            .await
            .map_err(|e| DownloadError::Provider(e.to_string()))?;

        // 1. Read initial probe chunk to detect audio container extension
        let mut initial_chunk = vec![0u8; 1024];
        let mut header_len = 0;
        while header_len < initial_chunk.len() {
            let n = source_stream
                .read(&mut initial_chunk[header_len..])
                .map_err(|e| DownloadError::Provider(format!("Stream read failure: {e}")))?;
            if n == 0 {
                break;
            }
            header_len += n;
        }
        initial_chunk.truncate(header_len);

        let ext = detect_audio_container(&initial_chunk);
        let artist_clean = sanitize_filename(&track.metadata.artist);
        let title_clean = sanitize_filename(&track.metadata.title);
        let base_name = format!("{artist_clean} - {title_clean}");
        let final_path = get_unique_filepath(destination_folder, &base_name, ext);
        let part_path = final_path.with_extension(format!("{ext}.part"));

        let mut output_file = File::create(&part_path)
            .await
            .map_err(|e| DownloadError::Io(format!("Disk file creation failure: {e}")))?;

        if !initial_chunk.is_empty() {
            output_file
                .write_all(&initial_chunk)
                .await
                .map_err(|e| DownloadError::Io(format!("Disk write failure: {e}")))?;
        }

        let mut transfer_buffer = [0u8; 32768];
        let mut _total_bytes = initial_chunk.len() as u64;

        loop {
            if let Some(ref cancel) = cancel_flag {
                if cancel.load(Ordering::SeqCst) {
                    drop(output_file);
                    let _ = tokio::fs::remove_file(&part_path).await;
                    return Err(DownloadError::Io("Download cancelled by user".to_string()));
                }
            }

            let bytes_read = source_stream
                .read(&mut transfer_buffer)
                .map_err(|e| DownloadError::Provider(format!("Stream read failure: {e}")))?;
            if bytes_read == 0 {
                break;
            }
            output_file
                .write_all(&transfer_buffer[..bytes_read])
                .await
                .map_err(|e| DownloadError::Io(format!("Disk write failure: {e}")))?;

            _total_bytes += bytes_read as u64;
            if let Some(tx) = event_tx {
                let _ = tx.send(PlayerEvent::DownloadProgress {
                    track_id: track.id,
                    percent: 50,
                    completed: false,
                });
            }
        }

        output_file
            .flush()
            .await
            .map_err(|e| DownloadError::Io(format!("File flush failure: {e}")))?;
        drop(output_file);

        // Rename .part to final path
        tokio::fs::rename(&part_path, &final_path)
            .await
            .map_err(|e| DownloadError::Io(format!("Failed finalizing file: {e}")))?;

        // Pure-Rust Metadata Injection via Lofty
        let path_clone = final_path.clone();
        let metadata = track.metadata.clone();

        let _ = tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut tagged_file = match lofty::probe::Probe::open(&path_clone)
                .map_err(|e| e.to_string())?
                .guess_file_type()
                .map_err(|e| e.to_string())?
                .read()
            {
                Ok(tf) => tf,
                Err(_) => return Ok(()),
            };

            let tag = match tagged_file.primary_tag_mut() {
                Some(primary) => primary,
                None => {
                    let tag_type = tagged_file.primary_tag_type();
                    tagged_file.insert_tag(Tag::new(tag_type));
                    match tagged_file.primary_tag_mut() {
                        Some(t) => t,
                        None => match tagged_file.first_tag_mut() {
                            Some(t) => t,
                            None => return Ok(()),
                        },
                    }
                }
            };

            tag.set_artist(metadata.artist);
            tag.set_title(metadata.title);
            if let Some(album) = metadata.album {
                tag.set_album(album);
            }

            let _ = tag.save_to_path(&path_clone, WriteOptions::default());
            Ok(())
        })
        .await;

        if let Some(tx) = event_tx {
            let _ = tx.send(PlayerEvent::DownloadProgress {
                track_id: track.id,
                percent: 100,
                completed: true,
            });
        }

        Ok(final_path)
    }
}

/// Service implementing [`TrackDownloadPort`].
pub struct TrackDownloadService {
    resolvers: Arc<RwLock<Vec<Arc<dyn StreamResolverPort>>>>,
    event_tx: Option<broadcast::Sender<PlayerEvent>>,
    active_jobs: Arc<Mutex<HashMap<TrackId, Arc<AtomicBool>>>>,
}

impl TrackDownloadService {
    pub fn new(
        resolvers: Vec<Arc<dyn StreamResolverPort>>,
        event_tx: Option<broadcast::Sender<PlayerEvent>>,
    ) -> Self {
        Self {
            resolvers: Arc::new(RwLock::new(resolvers)),
            event_tx,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_shared_resolvers(
        resolvers: Arc<RwLock<Vec<Arc<dyn StreamResolverPort>>>>,
        event_tx: Option<broadcast::Sender<PlayerEvent>>,
    ) -> Self {
        Self {
            resolvers,
            event_tx,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TrackDownloadPort for TrackDownloadService {
    async fn queue_download(
        &self,
        track: Track,
        target_directory: String,
    ) -> Result<String, DownloadError> {
        let resolvers = self.resolvers.read().await;
        let resolver = resolvers
            .iter()
            .find(|r| r.handles_source(&track.source))
            .cloned()
            .ok_or(DownloadError::UnsupportedSource)?;
        drop(resolvers);

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let track_id = track.id;
        {
            let mut jobs = self.active_jobs.lock().await;
            jobs.insert(track_id, cancel_flag.clone());
        }

        let event_tx = self.event_tx.clone();
        let dest_path = PathBuf::from(target_directory);
        let active_jobs = self.active_jobs.clone();

        let download_result = OfflineTrackDownloader::download_and_tag(
            resolver.as_ref(),
            &track,
            &dest_path,
            Some(cancel_flag),
            event_tx.as_ref(),
        )
        .await;

        {
            let mut jobs = active_jobs.lock().await;
            jobs.remove(&track_id);
        }

        let output_path = download_result?;
        Ok(output_path.to_string_lossy().to_string())
    }

    async fn cancel_download(&self, track_id: TrackId) -> Result<(), DownloadError> {
        let jobs = self.active_jobs.lock().await;
        if let Some(flag) = jobs.get(&track_id) {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            Err(DownloadError::NotFound(track_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};
    use amp_ports::{AudioSourceStream, ProviderError};
    use std::io::Cursor;

    struct MockResolver {
        data: Vec<u8>,
    }

    #[async_trait]
    impl StreamResolverPort for MockResolver {
        fn handles_source(&self, _source: &AudioSource) -> bool {
            true
        }

        async fn resolve_stream(
            &self,
            _source: &AudioSource,
        ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
            Ok(Box::new(Cursor::new(self.data.clone())))
        }
    }

    fn make_test_wav() -> Vec<u8> {
        let mut data = Vec::new();
        let sample_data = vec![0u8; 64];
        let file_size_minus_8 = (12 + 24 + 8 + sample_data.len() - 8) as u32;

        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&file_size_minus_8.to_le_bytes());
        data.extend_from_slice(b"WAVEfmt ");
        data.extend_from_slice(&(16u32).to_le_bytes());
        data.extend_from_slice(&(1u16).to_le_bytes());
        data.extend_from_slice(&(2u16).to_le_bytes());
        data.extend_from_slice(&(44100u32).to_le_bytes());
        data.extend_from_slice(&(176400u32).to_le_bytes());
        data.extend_from_slice(&(4u16).to_le_bytes());
        data.extend_from_slice(&(16u16).to_le_bytes());
        data.extend_from_slice(b"data");
        data.extend_from_slice(&(sample_data.len() as u32).to_le_bytes());
        data.extend_from_slice(&sample_data);
        data
    }

    #[test]
    fn test_sanitization_and_container_detection() {
        assert_eq!(
            sanitize_filename("AC/DC: Highway to Hell?"),
            "AC_DC_ Highway to Hell"
        );
        assert_eq!(sanitize_filename("CON"), "_CON");
        assert_eq!(sanitize_filename("../../../etc/passwd"), "etc_passwd");

        let wav_header = make_test_wav();
        assert_eq!(detect_audio_container(&wav_header), "wav");
        assert_eq!(detect_audio_container(b"fLaC\x00\x00"), "flac");
        assert_eq!(detect_audio_container(b"OggS\x00\x02"), "ogg");
        assert_eq!(detect_audio_container(b"\x00\x00\x00\x20ftypM4A "), "m4a");
        assert_eq!(detect_audio_container(b"ID3\x04\x00\x00"), "mp3");
    }

    #[test]
    fn test_unique_filepath_avoids_collision() {
        let temp_dir = std::env::temp_dir().join(format!("unique_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let path1 = get_unique_filepath(&temp_dir, "Track", "mp3");
        assert_eq!(path1, temp_dir.join("Track.mp3"));
        std::fs::write(&path1, b"1").unwrap();

        let path2 = get_unique_filepath(&temp_dir, "Track", "mp3");
        assert_eq!(path2, temp_dir.join("Track (2).mp3"));
        std::fs::write(&path2, b"2").unwrap();

        let path3 = get_unique_filepath(&temp_dir, "Track", "mp3");
        assert_eq!(path3, temp_dir.join("Track (3).mp3"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_download_and_tag_wav() {
        let temp_dir = std::env::temp_dir().join(format!("download_test_{}", uuid::Uuid::new_v4()));
        let track = Track::new(
            AudioSource::DirectHttp {
                url: "http://example.com/test.wav".to_string(),
            },
            TrackMetadata::new("Test Song", "Test Artist"),
        );

        let resolver = MockResolver {
            data: make_test_wav(),
        };

        let result =
            OfflineTrackDownloader::download_and_tag(&resolver, &track, &temp_dir, None, None)
                .await;

        assert!(result.is_ok(), "Download failed: {:?}", result.err());
        let file_path = result.unwrap();
        assert!(file_path.exists());
        assert_eq!(file_path.extension().and_then(|e| e.to_str()), Some("wav"));

        let probed = lofty::probe::Probe::open(&file_path)
            .unwrap()
            .guess_file_type()
            .unwrap()
            .read()
            .unwrap();
        let tag = probed
            .primary_tag()
            .or_else(|| probed.first_tag())
            .expect("Tag missing");
        assert_eq!(tag.artist().as_deref(), Some("Test Artist"));
        assert_eq!(tag.title().as_deref(), Some("Test Song"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_track_download_service_queue_and_cancel() {
        let temp_dir = std::env::temp_dir().join(format!("service_test_{}", uuid::Uuid::new_v4()));
        let track = Track::new(
            AudioSource::DirectHttp {
                url: "http://example.com/stream.wav".to_string(),
            },
            TrackMetadata::new("Downloader Song", "Downloader Artist"),
        );
        let track_id = track.id;

        let resolver = Arc::new(MockResolver {
            data: make_test_wav(),
        });
        let (event_tx, mut event_rx) = broadcast::channel(16);
        let service = TrackDownloadService::new(vec![resolver], Some(event_tx));

        let res = service
            .queue_download(track, temp_dir.to_string_lossy().to_string())
            .await;
        assert!(res.is_ok(), "Queue download failed: {:?}", res.err());
        let saved_path = PathBuf::from(res.unwrap());
        assert!(saved_path.exists());

        // Verify progress event was emitted
        let mut received_progress = false;
        while let Ok(event) = event_rx.try_recv() {
            if let PlayerEvent::DownloadProgress {
                track_id: id,
                completed,
                ..
            } = event
            {
                if id == track_id && completed {
                    received_progress = true;
                }
            }
        }
        assert!(
            received_progress,
            "Expected completed download progress event"
        );

        // Cancel on non-existent job returns NotFound
        let cancel_res = service.cancel_download(track_id).await;
        assert!(matches!(cancel_res, Err(DownloadError::NotFound(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
