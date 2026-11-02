//! The opt-in update check (TASK-183): which release to offer, whether it fixes an online
//! source, and that the check runs at most once a day.

#![forbid(unsafe_code)]

use amp_core_engine::update_check::{
    check_is_due, newer_release, ReleaseInfo, CHECK_INTERVAL_SECS,
};

fn release(tag: &str, prerelease: bool, body: &str) -> ReleaseInfo {
    ReleaseInfo {
        tag: tag.into(),
        name: format!("Multiplay {tag}"),
        prerelease,
        url: format!("https://github.com/el-j/multiplay/releases/tag/{tag}"),
        body: body.into(),
    }
}

#[test]
fn the_newest_newer_release_is_offered() {
    let releases = vec![
        release("v0.1.0-beta.1", true, ""),
        release("v0.1.0-beta.3", true, ""),
        release("v0.1.0-beta.2", true, ""),
        release("not-a-version", false, ""),
    ];
    let offer = newer_release("0.1.0-beta.1", &releases, true).expect("beta.3");
    assert_eq!(offer.tag, "v0.1.0-beta.3");
    assert!(
        newer_release("0.1.0-beta.3", &releases, true).is_none(),
        "already the newest"
    );
}

#[test]
fn betas_only_when_asked_and_a_final_beats_its_betas() {
    let releases = vec![
        release("v0.1.0-beta.9", true, ""),
        release("v0.1.0", false, ""),
    ];
    assert_eq!(
        newer_release("0.1.0-beta.2", &releases, false).unwrap().tag,
        "v0.1.0"
    );
    let only_betas = vec![release("v0.2.0-beta.1", true, "")];
    assert!(
        newer_release("0.1.0", &only_betas, false).is_none(),
        "no betas unless chosen"
    );
    assert!(newer_release("0.1.0", &only_betas, true).is_some());
}

#[test]
fn a_release_that_fixes_an_online_source_says_so() {
    let fix = release(
        "v0.1.1",
        false,
        "### Provider fix\nYouTube playback works again.",
    );
    let plain = release("v0.1.2", false, "Faster startup.");
    assert!(fix.fixes_online_source());
    assert!(!plain.fixes_online_source());
}

#[test]
fn the_check_runs_at_most_once_a_day() {
    let now = 1_800_000_000;
    assert!(check_is_due(0, now), "never checked");
    assert!(!check_is_due(now - 60, now));
    assert!(!check_is_due(now - CHECK_INTERVAL_SECS + 1, now));
    assert!(check_is_due(now - CHECK_INTERVAL_SECS, now));
    assert!(
        check_is_due(now + 3_600, now),
        "a clock set back does not block it forever"
    );
}
