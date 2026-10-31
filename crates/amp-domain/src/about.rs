//! How the app identifies itself: the `--version` line, the About tab and the HTTP User-Agent.

#![forbid(unsafe_code)]

/// Source code, issues and the privacy statement live here.
pub const REPOSITORY_URL: &str = "https://github.com/el-j/multiplay";

/// The privacy statement: every connection the app makes, and when.
pub const PRIVACY_URL: &str = "https://github.com/el-j/multiplay/blob/main/docs/privacy.md";

/// Bundled next to the executable in every release download.
pub const LICENSES_FILE_NAME: &str = "THIRD_PARTY_LICENSES.html";

/// `multiplay 0.1.0-beta.2 (1a2b3c4)`; the commit is left out when it is unknown (empty).
#[must_use]
pub fn version_line(app: &str, version: &str, commit: &str) -> String {
    if commit.is_empty() {
        format!("{app} {version}")
    } else {
        format!("{app} {version} ({commit})")
    }
}

/// The User-Agent sent with every HTTP request: the real version and where to find the project.
#[must_use]
pub fn user_agent() -> String {
    format!(
        "multiplay/{} (+{REPOSITORY_URL})",
        env!("CARGO_PKG_VERSION")
    )
}
