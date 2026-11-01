//! The opt-in update check in the app (TASK-183): off by default, at most once a day when on,
//! and a newer release shows up as a notice.

#![forbid(unsafe_code)]

use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::about::ReleaseInfo;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn newer() -> Vec<ReleaseInfo> {
    vec![ReleaseInfo {
        tag: "v99.0.0".into(),
        name: "Multiplay 99".into(),
        prerelease: false,
        url: "https://github.com/el-j/multiplay/releases/tag/v99.0.0".into(),
        body: "### Provider fix\nYouTube works again.".into(),
    }]
}

fn counting(
    calls: &Arc<AtomicUsize>,
) -> impl FnOnce() -> Result<Vec<ReleaseInfo>, String> + Send + 'static {
    let calls = Arc::clone(calls);
    move || {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(newer())
    }
}

fn settle(app: &mut MultiplayGuiApp) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.update_checking() {
        app.poll_update_check();
        assert!(Instant::now() < deadline, "update check never finished");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn off_by_default_nothing_is_asked() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    assert!(!app.config.updates.check_for_updates);
    let calls = Arc::new(AtomicUsize::new(0));
    app.maybe_check_for_updates_with(counting(&calls));
    settle(&mut app);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(app.update_available.is_none());
}

#[test]
fn when_on_it_asks_once_a_day_and_offers_the_newer_release() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    app.config.updates.check_for_updates = true;
    let calls = Arc::new(AtomicUsize::new(0));
    app.maybe_check_for_updates_with(counting(&calls));
    settle(&mut app);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let offer = app.update_available.clone().expect("a newer release");
    assert_eq!(offer.tag, "v99.0.0");
    assert!(
        app.config.updates.last_checked_epoch > 0,
        "remembered when it last asked"
    );

    // Asked again the same day: no request.
    app.maybe_check_for_updates_with(counting(&calls));
    settle(&mut app);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn a_failed_check_is_quiet_and_tried_again_tomorrow() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    app.config.updates.check_for_updates = true;
    app.maybe_check_for_updates_with(|| Err("offline".to_string()));
    settle(&mut app);
    assert!(app.update_available.is_none());
    assert!(app.config.updates.last_checked_epoch > 0);
}
