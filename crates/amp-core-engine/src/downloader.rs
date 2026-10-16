//! Offline stream downloader pipeline with pure-Rust Lofty metadata tagging.

#![forbid(unsafe_code)]

use amp_domain::Track;
use amp_ports::driven::StreamResolverPort;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::tag::{Accessor, Tag, TagExt};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct OfflineTrackDownloader;

impl OfflineTrackDownloader {
    /// Downloads an audio stream to destination disk folder and embeds Lofty ID3v2/Vorbis/RIFF metadata tags.
    pub async fn download_and_tag(
        resolver: &dyn StreamResolverPort,
        track: &Track,
        destination_folder: &Path,
    ) -> Result<PathBuf, String> {
        let mut source_stream = resolver
            .resolve_stream(&track.source)
            .await
            .map_err(|e| format!("Resolver error: {e}"))?;

        let sanitized_artist = track.metadata.artist.replace(['/', '\\'], "_");
        let sanitized_title = track.metadata.title.replace(['/', '\\'], "_");
        let file_name = format!("{sanitized_artist} - {sanitized_title}.wav");
        let output_path = destination_folder.join(file_name);

        let mut output_file = File::create(&output_path)
            .await
            .map_err(|e| format!("Disk file creation failure: {e}"))?;

        let mut transfer_buffer = [0u8; 32768];
        loop {
            let bytes_read = source_stream
                .read(&mut transfer_buffer)
                .map_err(|e| format!("Stream read failure: {e}"))?;
            if bytes_read == 0 {
                break;
            }
            output_file
                .write_all(&transfer_buffer[..bytes_read])
                .await
                .map_err(|e| format!("Disk write failure: {e}"))?;
        }
        output_file
            .flush()
            .await
            .map_err(|e| format!("File flush failure: {e}"))?;

        // Pure-Rust Metadata Injection via Lofty
        let path_clone = output_path.clone();
        let metadata = track.metadata.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut tagged_file = lofty::probe::Probe::open(&path_clone)
                .map_err(|e| e.to_string())?
                .guess_file_type()
                .map_err(|e| e.to_string())?
                .read()
                .map_err(|e| e.to_string())?;

            let tag = match tagged_file.primary_tag_mut() {
                Some(primary) => primary,
                None => {
                    let tag_type = tagged_file.primary_tag_type();
                    tagged_file.insert_tag(Tag::new(tag_type));
                    tagged_file.primary_tag_mut().unwrap()
                }
            };

            tag.set_artist(metadata.artist);
            tag.set_title(metadata.title);
            if let Some(album) = metadata.album {
                tag.set_album(album);
            }

            tag.save_to_path(&path_clone, WriteOptions::default())
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| format!("Tagging thread panic: {e}"))?
        .map_err(|e| format!("Metadata tagging failed: {e}"))?;

        Ok(output_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};
    use amp_ports::{AudioSourceStream, ProviderError};
    use async_trait::async_trait;
    use std::io::Cursor;

    struct MockResolver {
        wav_bytes: Vec<u8>,
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
            Ok(Box::new(Cursor::new(self.wav_bytes.clone())))
        }
    }

    fn make_test_wav() -> Vec<u8> {
        let mut data = Vec::new();
        let sample_data = vec![0u8; 64];
        let file_size_minus_8 = (12 + 24 + 8 + sample_data.len() - 8) as u32;

        // RIFF header
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&file_size_minus_8.to_le_bytes());
        data.extend_from_slice(b"WAVE");

        // fmt chunk (24 bytes total)
        data.extend_from_slice(b"fmt ");
        data.extend_from_slice(&(16u32).to_le_bytes());
        data.extend_from_slice(&(1u16).to_le_bytes()); // PCM
        data.extend_from_slice(&(2u16).to_le_bytes()); // 2 channels
        data.extend_from_slice(&(44100u32).to_le_bytes()); // 44100 Hz
        data.extend_from_slice(&(176400u32).to_le_bytes()); // byte rate
        data.extend_from_slice(&(4u16).to_le_bytes()); // block align
        data.extend_from_slice(&(16u16).to_le_bytes()); // 16 bits

        // data chunk
        data.extend_from_slice(b"data");
        data.extend_from_slice(&(sample_data.len() as u32).to_le_bytes());
        data.extend_from_slice(&sample_data);
        data
    }

    #[tokio::test]
    async fn test_download_and_tag() {
        let temp_dir = std::env::temp_dir().join(format!("multiplay_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let track = Track::new(
            AudioSource::DirectHttp {
                url: "http://example.com/test.wav".to_string(),
            },
            TrackMetadata::new("Test Song", "Test Artist"),
        );

        let resolver = MockResolver {
            wav_bytes: make_test_wav(),
        };

        let result = OfflineTrackDownloader::download_and_tag(&resolver, &track, &temp_dir).await;
        assert!(result.is_ok(), "Download and tag failed: {:?}", result.err());

        let file_path = result.unwrap();
        assert!(file_path.exists());

        // Verify tagged file using lofty
        let probed = lofty::probe::Probe::open(&file_path)
            .unwrap()
            .guess_file_type()
            .unwrap()
            .read()
            .unwrap();
        let tag = probed.primary_tag().or_else(|| probed.first_tag()).expect("Tag missing");
        assert_eq!(tag.artist().as_deref(), Some("Test Artist"));
        assert_eq!(tag.title().as_deref(), Some("Test Song"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
