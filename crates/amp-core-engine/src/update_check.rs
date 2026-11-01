//! Deciding about updates (TASK-183): the newest release newer than this one, whether it fixes
//! an online source, and the once-a-day schedule. Fetching the list is the HTTP adapter's job;
//! the check is off unless the user turns it on.

#![forbid(unsafe_code)]

use semver::Version;

/// Seconds between two checks: once a day at most.
pub const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

pub use amp_domain::about::ReleaseInfo;

fn version_of(r: &ReleaseInfo) -> Option<Version> {
    Version::parse(r.tag.trim_start_matches('v')).ok()
}

/// The newest release newer than `current`; betas only when `include_betas`.
#[must_use]
pub fn newer_release(
    current: &str,
    releases: &[ReleaseInfo],
    include_betas: bool,
) -> Option<ReleaseInfo> {
    let current = Version::parse(current.trim_start_matches('v')).ok()?;
    releases
        .iter()
        .filter(|r| include_betas || !r.prerelease)
        .filter_map(|r| Some((version_of(r)?, r)))
        .filter(|(v, _)| *v > current)
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, r)| r.clone())
}

/// True when a day has passed since `last_checked` (Unix seconds; 0 = never), or the clock
/// went backwards.
#[must_use]
pub fn check_is_due(last_checked: u64, now: u64) -> bool {
    last_checked == 0 || now < last_checked || now - last_checked >= CHECK_INTERVAL_SECS
}
