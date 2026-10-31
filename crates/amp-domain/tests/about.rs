//! What the app says about itself (TASK-180, TASK-193): one version line for `--version` and the
//! About tab, and one honest User-Agent for every HTTP request.

#![forbid(unsafe_code)]

use amp_domain::about::{user_agent, version_line, REPOSITORY_URL};

#[test]
fn the_version_line_names_the_app_the_version_and_the_commit() {
    assert_eq!(
        version_line("multiplay", "0.1.0-beta.2", "1a2b3c4"),
        "multiplay 0.1.0-beta.2 (1a2b3c4)"
    );
}

#[test]
fn an_unknown_commit_is_left_out() {
    assert_eq!(version_line("multiplay", "0.1.0", ""), "multiplay 0.1.0");
}

#[test]
fn the_user_agent_carries_the_real_version_and_a_contact_url() {
    let ua = user_agent();
    assert!(
        ua.starts_with(&format!("multiplay/{} ", env!("CARGO_PKG_VERSION"))),
        "{ua}"
    );
    assert!(ua.contains(REPOSITORY_URL), "{ua}");
    assert!(
        !ua.contains("Windows NT"),
        "no pretending to be a browser: {ua}"
    );
}
