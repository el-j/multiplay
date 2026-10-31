//! Config files written by earlier releases (serialized with `basic-toml`) must keep loading
//! after the switch to the `toml` crate, and what we write now must load again.

#![forbid(unsafe_code)]

use amp_core_engine::ConfigStore;
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic_toml_config.toml")
}

#[test]
fn config_written_by_basic_toml_still_loads() {
    let cfg = ConfigStore::load_or_default(fixture(), None).current();
    assert_eq!(cfg.audio.sample_rate, 96000);
    assert_eq!(cfg.audio.target_lufs, -14);
    assert!(cfg.ui.compact_mode);
    assert_eq!(
        cfg.library.music_directories,
        vec!["/Music/My \"Quoted\" Folder".to_string()]
    );
    assert_eq!(
        cfg.plugins.get("visualizer").map(String::as_str),
        Some("on")
    );
    assert!(cfg.remote.enabled);
    assert_eq!(cfg.remote.port, 9100);
}

#[test]
fn rewritten_config_round_trips() {
    let dir = std::env::temp_dir().join(format!("multiplay-config-{}", uuid::Uuid::new_v4()));
    let path = dir.join("multiplay.toml");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture(), &path).unwrap();

    let store = ConfigStore::load_or_default(&path, None);
    let before = store.current();
    store.update(|c| c.audio.buffer_frames = 2048).unwrap();
    let after = ConfigStore::load_or_default(&path, None).current();
    assert_eq!(after.audio.buffer_frames, 2048);
    assert_eq!(after.library, before.library);
    assert_eq!(after.plugins, before.plugins);
    assert_eq!(after.remote, before.remote);
    let _ = std::fs::remove_dir_all(dir);
}
