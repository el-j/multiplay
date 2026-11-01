//! "Use Soulseek" in the app (TASK-162): off by default; on, it connects to the user's slskd or
//! asks to install one, and says what is going on.

#![forbid(unsafe_code)]

use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::config::SoulseekMode;
use std::time::{Duration, Instant};

fn settle(app: &mut MultiplayGuiApp) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.soulseek_busy() {
        app.poll_soulseek();
        assert!(Instant::now() < deadline, "Soulseek setup never finished");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn off_by_default_and_not_searched() {
    let dir = tempfile::tempdir().unwrap();
    let app = MultiplayGuiApp::headless(dir.path()).unwrap();
    assert!(!app.config.soulseek.enabled);
    assert!(!app.enabled_providers().contains(&"soulseek"));
}

#[test]
fn an_unreachable_slskd_is_reported_and_not_searched() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    app.config.soulseek.enabled = true;
    app.config.soulseek.mode = SoulseekMode::External;
    app.config.soulseek.url = "http://127.0.0.1:1".into();
    app.apply_soulseek();
    settle(&mut app);
    assert!(
        app.soulseek_status().contains("not running"),
        "{}",
        app.soulseek_status()
    );
    assert!(
        !app.enabled_providers().contains(&"soulseek"),
        "only searched once connected"
    );
}

#[test]
fn managed_mode_without_slskd_asks_to_install_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    app.config.soulseek.enabled = true;
    app.config.soulseek.mode = SoulseekMode::Managed;
    app.apply_soulseek();
    settle(&mut app);
    assert!(
        app.soulseek_status().contains("Install"),
        "{}",
        app.soulseek_status()
    );
    assert!(!app.slskd_installed());
}

#[test]
fn secrets_go_to_the_vault_not_the_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    app.save_soulseek_secret("soulseek_password", "hunter2");
    assert_eq!(
        app.soulseek_secret("soulseek_password").as_deref(),
        Some("hunter2")
    );
    app.config.soulseek.username = "listener".into();
    app.save_and_apply_config();
    let on_disk: String = walk(dir.path())
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "toml" || e == "json"))
        .map(|p| std::fs::read_to_string(p).unwrap_or_default())
        .collect();
    assert!(
        !on_disk.contains("hunter2"),
        "the password is not in any settings file"
    );
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}
