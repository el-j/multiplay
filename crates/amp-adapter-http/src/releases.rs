//! The release list for the opt-in update check (TASK-183): one GET to GitHub's public API for
//! this project's releases. Nothing about the user or their library is sent.

#![forbid(unsafe_code)]

use amp_domain::about::ReleaseInfo;
use serde::Deserialize;
use std::time::Duration;

/// The only address the update check asks.
pub const RELEASES_URL: &str = "https://api.github.com/repos/el-j/multiplay/releases?per_page=20";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    body: Option<String>,
}

/// Published releases in GitHub's answer; drafts are skipped.
///
/// # Errors
/// When the answer is not a release list (rate limit, outage, garbage).
pub fn parse_releases(json: &str) -> Result<Vec<ReleaseInfo>, String> {
    let list: Vec<GithubRelease> =
        serde_json::from_str(json).map_err(|e| format!("not a release list: {e}"))?;
    Ok(list
        .into_iter()
        .filter(|r| !r.draft)
        .map(|r| ReleaseInfo {
            name: r
                .name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| r.tag_name.clone()),
            tag: r.tag_name,
            prerelease: r.prerelease,
            url: r.html_url,
            body: r.body.unwrap_or_default(),
        })
        .collect())
}

/// Fetches the release list (blocking; call it off the UI thread).
///
/// # Errors
/// Network failures and answers that are not a release list.
pub fn fetch_releases() -> Result<Vec<ReleaseInfo>, String> {
    crate::install_ring_provider();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent(amp_domain::about::user_agent())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        let response = client
            .get(RELEASES_URL)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("update check: {e}"))?;
        let text = response
            .text()
            .await
            .map_err(|e| format!("update check: {e}"))?;
        parse_releases(&text)
    })
}
