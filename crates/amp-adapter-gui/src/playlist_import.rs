//! Playlists and CUE sheets added by drop, dialog or folder scan (TASK-232). The import itself
//! lives in `amp_adapter_storage::import`, shared with the TUI.

#![forbid(unsafe_code)]

/// Extensions of files that list tracks rather than hold audio.
pub use amp_adapter_storage::import::LIST_EXTENSIONS;
