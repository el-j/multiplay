//! Minimal plain-HTTP responses served next to the WebSocket endpoint.

#![forbid(unsafe_code)]

/// Security headers sent with every plain-HTTP response. No CORS headers are ever sent.
const SECURITY_HEADERS: &str = "X-Content-Type-Options: nosniff\r\n\
Referrer-Policy: no-referrer\r\n\
Cache-Control: no-store\r\n\
X-Frame-Options: DENY\r\n\
Content-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n";

/// Routes a non-upgrade request (first line of the request) to a full HTTP response.
#[must_use]
pub fn respond(request_line: &str) -> String {
    let mut parts = request_line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let path = target.split('?').next().unwrap_or("");
    match (method, path) {
        ("GET", "/" | "/index.html") => build(
            "200 OK",
            "text/html; charset=utf-8",
            crate::web_client::MOBILE_WEB_CLIENT_HTML,
        ),
        ("GET", "/api/status") => build(
            "200 OK",
            "application/json",
            r#"{"status":"ok","service":"multiplay-remote"}"#,
        ),
        ("GET", _) => build("404 Not Found", "text/plain; charset=utf-8", "not found"),
        _ => build(
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            "method not allowed",
        ),
    }
}

fn build(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{SECURITY_HEADERS}Connection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_known_paths_and_404s_the_rest() {
        assert!(respond("GET /?token=AB HTTP/1.1").starts_with("HTTP/1.1 200 OK"));
        assert!(respond("GET /api/status HTTP/1.1").contains("multiplay-remote"));
        assert!(respond("GET /../etc/passwd HTTP/1.1").starts_with("HTTP/1.1 404"));
        assert!(respond("POST / HTTP/1.1").starts_with("HTTP/1.1 405"));
    }
}
