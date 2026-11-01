//! The release list for the opt-in update check (TASK-183): GitHub's answer parsed, drafts
//! skipped, missing fields tolerated, garbage refused.

#![forbid(unsafe_code)]

use amp_adapter_http::releases::{parse_releases, RELEASES_URL};

#[test]
fn github_releases_parse_and_drafts_are_skipped() {
    let releases = parse_releases(include_str!("fixtures/github_releases.json")).unwrap();
    let tags: Vec<_> = releases.iter().map(|r| r.tag.as_str()).collect();
    assert_eq!(tags, ["v0.1.0-beta.3", "v0.1.0-beta.2"]);
    assert!(releases[0].prerelease && releases[0].fixes_online_source());
    assert_eq!(
        releases[1].name, "v0.1.0-beta.2",
        "a missing name falls back to the tag"
    );
    assert!(releases[0]
        .url
        .starts_with("https://github.com/el-j/multiplay/releases/"));
}

#[test]
fn garbage_is_an_error() {
    assert!(parse_releases("{\"message\": \"API rate limit exceeded\"}").is_err());
    assert!(parse_releases("<html>").is_err());
}

#[test]
fn only_the_projects_own_release_list_is_asked() {
    assert!(RELEASES_URL.starts_with("https://api.github.com/repos/el-j/multiplay/releases"));
}

#[test]
#[ignore = "live network: asks api.github.com"]
fn the_live_release_list_parses() {
    let releases =
        amp_adapter_http::releases::fetch_releases().expect("GitHub answers with a list");
    println!("{} published releases", releases.len());
}
