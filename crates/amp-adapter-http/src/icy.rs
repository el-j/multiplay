//! ICY (Icecast / Shoutcast) inline metadata demultiplexer (`TASK-137`).
//!
//! Strips `icy-metaint` metadata blocks out of an HTTP audio byte stream so the
//! downstream decoder receives pure audio frames while extracting `StreamTitle`
//! updates for `PlayerEvent::NowPlayingMetadata`.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

/// Song titles a stream remembers.
pub const MAX_TITLE_HISTORY: usize = 100;

/// Shared sink for the most recent ICY `StreamTitle` and history of titles observed.
#[derive(Debug, Default, Clone)]
pub struct IcyMetadataTap {
    inner: Arc<Mutex<IcyMetadataState>>,
}

#[derive(Debug, Default)]
struct IcyMetadataState {
    latest_title: Option<String>,
    history: Vec<String>,
}

impl IcyMetadataTap {
    /// Creates an empty [`IcyMetadataTap`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a newly parsed `StreamTitle`.
    pub fn record_title(&self, title: String) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.latest_title.as_deref() != Some(title.as_str()) {
                guard.latest_title = Some(title.clone());
                guard.history.push(title);
                // A station running for days must not grow this forever (TASK-185).
                if guard.history.len() > MAX_TITLE_HISTORY {
                    let excess = guard.history.len() - MAX_TITLE_HISTORY;
                    guard.history.drain(..excess);
                }
            }
        }
    }

    /// Returns the most recently observed `StreamTitle`.
    pub fn latest_title(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|g| g.latest_title.clone())
    }

    /// Returns all distinct `StreamTitle` updates observed in order.
    pub fn title_history(&self) -> Vec<String> {
        self.inner
            .lock()
            .map(|g| g.history.clone())
            .unwrap_or_default()
    }
}

/// Stateful demultiplexer that separates audio bytes from ICY metadata blocks
/// occurring every `metaint` audio bytes.
pub struct IcyDemuxer {
    metaint: usize,
    audio_remaining: usize,
    meta_remaining: usize,
    reading_length_byte: bool,
    meta_buf: Vec<u8>,
    tap: IcyMetadataTap,
}

impl IcyDemuxer {
    /// Creates a new [`IcyDemuxer`] for a stream with `icy-metaint = metaint`.
    pub fn new(metaint: usize, tap: IcyMetadataTap) -> Self {
        let metaint = metaint.max(1);
        Self {
            metaint,
            audio_remaining: metaint,
            meta_remaining: 0,
            reading_length_byte: false,
            meta_buf: Vec::with_capacity(256),
            tap,
        }
    }

    /// Returns a handle to the metadata tap.
    pub fn tap(&self) -> IcyMetadataTap {
        self.tap.clone()
    }

    /// Pushes raw HTTP response bytes through the demuxer and appends pure audio
    /// bytes into `audio_out`. Any complete ICY metadata blocks are parsed and
    /// published to the [`IcyMetadataTap`].
    pub fn push_bytes(&mut self, mut input: &[u8], audio_out: &mut Vec<u8>) {
        while !input.is_empty() {
            if self.reading_length_byte {
                let len_blocks = input[0] as usize;
                input = &input[1..];
                self.reading_length_byte = false;
                self.meta_remaining = len_blocks * 16;
                self.meta_buf.clear();
                if self.meta_remaining == 0 {
                    self.audio_remaining = self.metaint;
                }
            } else if self.meta_remaining > 0 {
                let take = input.len().min(self.meta_remaining);
                self.meta_buf.extend_from_slice(&input[..take]);
                input = &input[take..];
                self.meta_remaining -= take;
                if self.meta_remaining == 0 {
                    if let Some(title) = parse_icy_stream_title(&self.meta_buf) {
                        self.tap.record_title(title);
                    }
                    self.meta_buf.clear();
                    self.audio_remaining = self.metaint;
                }
            } else {
                let take = input.len().min(self.audio_remaining);
                audio_out.extend_from_slice(&input[..take]);
                input = &input[take..];
                self.audio_remaining -= take;
                if self.audio_remaining == 0 {
                    self.reading_length_byte = true;
                }
            }
        }
    }
}

/// Extracts `StreamTitle='...'` from a raw ICY metadata block (padded with NUL bytes).
pub fn parse_icy_stream_title(raw_block: &[u8]) -> Option<String> {
    let trimmed_end = raw_block
        .iter()
        .rposition(|&b| b != 0)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    if trimmed_end == 0 {
        return None;
    }
    let text = String::from_utf8_lossy(&raw_block[..trimmed_end]);
    let marker = "StreamTitle='";
    let start = text.find(marker)? + marker.len();
    let rest = &text[start..];
    // ICY fields end with `';` (or a trailing `'` at the end of the block).
    let end = rest
        .find("';")
        .or_else(|| rest.rfind('\''))
        .unwrap_or(rest.len());
    let title = rest[..end].trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

/// Builds a synthetic ICY stream interleaving `audio` chunks of `metaint` bytes
/// with ICY metadata blocks from `titles` (cycling through `titles`).
pub fn build_synthetic_icy_stream(audio: &[u8], metaint: usize, titles: &[&str]) -> Vec<u8> {
    let mut out = Vec::with_capacity(audio.len() + 256);
    let mut pos = 0;
    let mut step = 0;
    while pos < audio.len() {
        let take = (audio.len() - pos).min(metaint);
        out.extend_from_slice(&audio[pos..pos + take]);
        pos += take;
        if take == metaint {
            if titles.is_empty() {
                out.push(0);
            } else {
                let title = titles[step % titles.len()];
                step += 1;
                let meta_str = format!("StreamTitle='{title}';");
                let meta_bytes = meta_str.as_bytes();
                let blocks = meta_bytes.len().div_ceil(16);
                out.push(blocks as u8);
                out.extend_from_slice(meta_bytes);
                let pad = blocks * 16 - meta_bytes.len();
                out.extend(std::iter::repeat_n(0u8, pad));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icy_demuxer_strips_metadata_and_extracts_titles_across_arbitrary_chunk_boundaries() {
        let metaint = 64;
        let original_audio: Vec<u8> = (0..1000).map(|i| (i % 251) as u8).collect();
        let titles = [
            "Boards of Canada - Roygbiv",
            "Aphex Twin - Xtal",
            "Tycho - Awake",
        ];
        let muxed = build_synthetic_icy_stream(&original_audio, metaint, &titles);

        let tap = IcyMetadataTap::new();
        let mut demux = IcyDemuxer::new(metaint, tap.clone());
        let mut recovered_audio = Vec::new();

        // Feed in prime-sized 13-byte chunks to stress state transitions mid-metadata
        for chunk in muxed.chunks(13) {
            demux.push_bytes(chunk, &mut recovered_audio);
        }

        assert_eq!(
            recovered_audio, original_audio,
            "Demuxed audio must be byte-for-byte identical to original audio"
        );
        let history = tap.title_history();
        assert!(history.len() >= 3);
        assert_eq!(history[0], "Boards of Canada - Roygbiv");
        assert_eq!(history[1], "Aphex Twin - Xtal");
        assert_eq!(history[2], "Tycho - Awake");
    }
}
