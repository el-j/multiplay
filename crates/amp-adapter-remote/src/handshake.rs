//! Pre-upgrade request checks: credentials, Host (DNS rebinding) and Origin (CSRF).

#![forbid(unsafe_code)]

use std::net::IpAddr;
use std::time::Duration;
use tokio_tungstenite::tungstenite::handshake::server::ErrorResponse;
use tokio_tungstenite::tungstenite::http::{header, Response, StatusCode};

/// Credential presented in the WebSocket URL query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    /// Single-use pairing token from the QR code (`?token=`).
    PairingToken(String),
    /// Session secret granted at pairing (`?session=`).
    Session(String),
}

/// Why an upgrade request was refused before the WebSocket handshake completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// Missing, unknown, expired or revoked credential.
    Unauthorized,
    /// The Host header is not an IP literal or localhost (DNS rebinding defence).
    ForbiddenHost,
    /// The Origin header does not match the daemon's own origin.
    ForbiddenOrigin,
    /// Wrong request path.
    NotFound,
    /// Too many failed attempts from this address.
    Throttled(Duration),
    /// The connection limit is reached.
    Busy,
}

impl Rejection {
    /// HTTP status sent for this rejection.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::ForbiddenHost | Self::ForbiddenOrigin => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Throttled(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Busy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Builds the HTTP error response returned instead of upgrading.
    #[must_use]
    pub fn into_response(self) -> ErrorResponse {
        let mut builder = Response::builder()
            .status(self.status())
            .header(header::CACHE_CONTROL, "no-store");
        if let Self::Throttled(retry) = self {
            builder = builder.header(header::RETRY_AFTER, retry.as_secs().max(1).to_string());
        }
        builder.body(None).unwrap_or_else(|_| Response::new(None))
    }
}

/// Extracts a credential from a raw query string such as `token=ABCD` or `session=...`.
#[must_use]
pub fn credential_from_query(query: Option<&str>) -> Option<Credential> {
    query?.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if value.is_empty() {
            return None;
        }
        match key {
            "token" => Some(Credential::PairingToken(value.to_string())),
            "session" => Some(Credential::Session(value.to_string())),
            _ => None,
        }
    })
}

/// Accepts only IP-literal hosts (optionally bracketed IPv6) and `localhost`, with optional port.
/// Browsers send the attacker's domain here during DNS rebinding, so names are refused.
#[must_use]
pub fn host_is_acceptable(host: &str) -> bool {
    let host = host.trim();
    let name = if let Some(rest) = host.strip_prefix('[') {
        match rest.split_once(']') {
            Some((v6, tail)) if tail.is_empty() || port_suffix_ok(tail) => v6,
            _ => return false,
        }
    } else {
        match host.rsplit_once(':') {
            Some((name, port)) if port_suffix_ok(&format!(":{port}")) => name,
            Some(_) => return false,
            None => host,
        }
    };
    name.eq_ignore_ascii_case("localhost") || name.parse::<IpAddr>().is_ok()
}

fn port_suffix_ok(tail: &str) -> bool {
    tail.strip_prefix(':')
        .is_some_and(|p| !p.is_empty() && p.parse::<u16>().is_ok())
}

/// Browsers always send `Origin` on WebSocket upgrades; it must be the daemon's own origin.
/// Non-browser clients may omit it (they still need a credential).
#[must_use]
pub fn origin_matches_host(origin: Option<&str>, host: &str) -> bool {
    match origin {
        None => true,
        Some(origin) => {
            let origin = origin.trim();
            origin.eq_ignore_ascii_case(&format!("http://{host}"))
                || origin.eq_ignore_ascii_case(&format!("https://{host}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_credentials() {
        assert_eq!(
            credential_from_query(Some("token=AB12")),
            Some(Credential::PairingToken("AB12".into()))
        );
        assert_eq!(
            credential_from_query(Some("x=1&session=ff")),
            Some(Credential::Session("ff".into()))
        );
        assert_eq!(credential_from_query(Some("token=")), None);
        assert_eq!(credential_from_query(None), None);
    }

    #[test]
    fn host_validation_blocks_rebinding_names() {
        for ok in [
            "127.0.0.1:9002",
            "192.168.1.20",
            "localhost:9002",
            "[::1]:9002",
            "[fe80::1]",
        ] {
            assert!(host_is_acceptable(ok), "{ok}");
        }
        for bad in [
            "evil.example:9002",
            "evil.example",
            "127.0.0.1:x",
            "[::1]x",
            "",
            "1.2.3.4:99999",
        ] {
            assert!(!host_is_acceptable(bad), "{bad}");
        }
    }

    #[test]
    fn origin_must_match_host() {
        assert!(origin_matches_host(None, "127.0.0.1:9002"));
        assert!(origin_matches_host(
            Some("http://127.0.0.1:9002"),
            "127.0.0.1:9002"
        ));
        assert!(!origin_matches_host(
            Some("http://evil.example"),
            "127.0.0.1:9002"
        ));
        assert!(!origin_matches_host(Some("null"), "127.0.0.1:9002"));
    }

    #[test]
    fn throttled_response_carries_retry_after() {
        let resp = Rejection::Throttled(Duration::from_secs(8)).into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(resp.headers()[header::RETRY_AFTER], "8");
    }
}
