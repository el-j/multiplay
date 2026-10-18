//! Versioned JSON WebSocket protocol for remote control and mobile clients.
//!
//! Every client message is an envelope `{"v":1,"type":"<Action>","payload":...}`.
//! Remotes can only reference tracks the host already knows (by [`TrackId`]);
//! they can never hand the host a file path or URL to open.

#![forbid(unsafe_code)]

use amp_domain::{Track, TrackId};
use serde::{Deserialize, Serialize};

/// Protocol version spoken by this daemon. Messages with any other `v` are rejected.
pub const PROTOCOL_VERSION: u32 = 1;

/// Inbound control action received from a connected remote client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RemoteAction {
    /// Request the current playback status.
    GetPlaybackStatus,
    /// Resume or begin playback.
    Play,
    /// Pause ongoing playback.
    Pause,
    /// Skip to the next track in the queue.
    Next,
    /// Return to the previous track.
    Previous,
    /// Adjust player output volume (0.0 to 1.0).
    SetVolume(f32),
    /// Seek to a specified position in seconds.
    Seek(u64),
    /// Enqueue a track the host already holds in its playlist or library.
    EnqueueById(TrackId),
    /// Cast a vote to skip the current track.
    VoteSkip,
}

/// Versioned envelope wrapping every client message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientMessage {
    /// Protocol version; must equal [`PROTOCOL_VERSION`].
    pub v: u32,
    /// The requested action.
    #[serde(flatten)]
    pub action: RemoteAction,
}

impl ClientMessage {
    /// Wraps an action in an envelope for the current protocol version.
    #[must_use]
    pub fn new(action: RemoteAction) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            action,
        }
    }
}

/// Reasons a client message is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Not valid JSON, missing fields, or an unknown action.
    Malformed(String),
    /// The envelope names a protocol version this daemon does not speak.
    UnsupportedVersion(u32),
    /// The action parsed but carries an out-of-range value.
    InvalidValue(&'static str),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(e) => write!(f, "malformed message: {e}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported protocol version {v}"),
            Self::InvalidValue(what) => write!(f, "invalid value: {what}"),
        }
    }
}

#[derive(Deserialize)]
struct VersionProbe {
    v: Option<u32>,
}

/// Parses and validates one client text frame. Never panics on any input.
pub fn parse_client_message(text: &str) -> Result<RemoteAction, ProtocolError> {
    let probe: VersionProbe =
        serde_json::from_str(text).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
    match probe.v {
        None => {
            return Err(ProtocolError::Malformed(
                "missing protocol version `v`".into(),
            ))
        }
        Some(PROTOCOL_VERSION) => {}
        Some(other) => return Err(ProtocolError::UnsupportedVersion(other)),
    }
    let msg: ClientMessage =
        serde_json::from_str(text).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
    if let RemoteAction::SetVolume(v) = msg.action {
        if !v.is_finite() {
            return Err(ProtocolError::InvalidValue(
                "volume must be a finite number",
            ));
        }
    }
    Ok(msg.action)
}

/// Track details shared with remotes. Deliberately omits the source path or URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteTrackInfo {
    /// Host-side identifier, usable with [`RemoteAction::EnqueueById`].
    pub id: TrackId,
    /// Track title.
    pub title: String,
    /// Track artist.
    pub artist: String,
    /// Duration in whole seconds, when known.
    pub duration_secs: Option<u64>,
}

impl From<&Track> for RemoteTrackInfo {
    fn from(track: &Track) -> Self {
        Self {
            id: track.id,
            title: track.metadata.title.clone(),
            artist: track.metadata.artist.clone(),
            duration_secs: track.metadata.duration.map(|d| d.as_secs()),
        }
    }
}

/// Outbound event pushed from the player to remote clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RemoteEventBroadcast {
    /// Snapshot of the player's active state.
    Status {
        current_track: Option<RemoteTrackInfo>,
        is_playing: bool,
        volume: f32,
        position_secs: u64,
        active_peers: usize,
    },
    /// Sent once after pairing: a long-lived secret the client stores to reconnect.
    SessionGranted { session: String },
    /// Acknowledgment of an applied action.
    ActionAck { action: String },
    /// Error notification (for example a refused message).
    Error { message: String },
}

/// Versioned envelope wrapping every server message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerMessage {
    /// Protocol version of the daemon.
    pub v: u32,
    /// The event.
    #[serde(flatten)]
    pub event: RemoteEventBroadcast,
}

/// Serializes an outbound event inside a versioned envelope.
#[must_use]
pub fn encode_server_message(event: &RemoteEventBroadcast) -> String {
    let msg = ServerMessage {
        v: PROTOCOL_VERSION,
        event: event.clone(),
    };
    serde_json::to_string(&msg).unwrap_or_else(|_| {
        format!(r#"{{"v":{PROTOCOL_VERSION},"type":"Error","payload":{{"message":"encode"}}}}"#)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_roundtrip_for_every_action() {
        let actions = [
            RemoteAction::GetPlaybackStatus,
            RemoteAction::Play,
            RemoteAction::Pause,
            RemoteAction::Next,
            RemoteAction::Previous,
            RemoteAction::SetVolume(0.25),
            RemoteAction::Seek(42),
            RemoteAction::EnqueueById(TrackId::new()),
            RemoteAction::VoteSkip,
        ];
        for action in actions {
            let json = serde_json::to_string(&ClientMessage::new(action.clone())).unwrap();
            assert_eq!(parse_client_message(&json), Ok(action), "{json}");
        }
    }

    #[test]
    fn rejects_missing_and_unknown_versions() {
        assert!(matches!(
            parse_client_message(r#"{"type":"Play"}"#),
            Err(ProtocolError::Malformed(_))
        ));
        assert_eq!(
            parse_client_message(r#"{"v":2,"type":"Play"}"#),
            Err(ProtocolError::UnsupportedVersion(2))
        );
    }

    #[test]
    fn rejects_legacy_track_enqueue() {
        let legacy =
            r#"{"v":1,"type":"EnqueueTrack","payload":{"source":{"LocalFile":{"path":"/x"}}}}"#;
        assert!(matches!(
            parse_client_message(legacy),
            Err(ProtocolError::Malformed(_))
        ));
    }

    #[test]
    fn never_panics_on_arbitrary_input() {
        // Deterministic xorshift corpus: random bytes plus mutated valid messages.
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let seeds = [
            r#"{"v":1,"type":"SetVolume","payload":0.5}"#,
            r#"{"v":1,"type":"Seek","payload":1}"#,
        ];
        for i in 0..5_000 {
            let mut bytes: Vec<u8> = if i % 2 == 0 {
                seeds[i % seeds.len()].as_bytes().to_vec()
            } else {
                (0..(next() % 64)).map(|_| next() as u8).collect()
            };
            if !bytes.is_empty() {
                let idx = (next() as usize) % bytes.len();
                bytes[idx] = next() as u8;
            }
            let _ = parse_client_message(&String::from_utf8_lossy(&bytes));
        }
    }

    #[test]
    fn server_messages_are_versioned_and_hide_sources() {
        let track = Track::new(
            amp_domain::AudioSource::LocalFile {
                path: "/home/me/secret.flac".into(),
            },
            amp_domain::TrackMetadata::new("Title", "Artist"),
        );
        let json = encode_server_message(&RemoteEventBroadcast::Status {
            current_track: Some(RemoteTrackInfo::from(&track)),
            is_playing: true,
            volume: 0.5,
            position_secs: 3,
            active_peers: 1,
        });
        assert!(json.starts_with(r#"{"v":1,"#));
        assert!(!json.contains("secret.flac"));
    }
}
