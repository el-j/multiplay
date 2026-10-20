//! The phone web client (TASK-217): served with a strict CSP, loads nothing from anywhere
//! else, speaks protocol v2 (search, paste a link, request, live request status), and never
//! turns host-supplied text (track titles) into markup.

#![forbid(unsafe_code)]

mod common;

use amp_adapter_remote::MOBILE_WEB_CLIENT_HTML;
use common::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut tcp = TcpStream::connect(addr).await.unwrap();
    tcp.write_all(format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut buf = Vec::new();
    tcp.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).into_owned()
}

#[tokio::test]
async fn web_page_is_served_with_a_strict_csp() {
    let h = start().await;
    let resp = get(h.addr, "/?token=ABCD").await;
    let head = resp.split("\r\n\r\n").next().unwrap().to_lowercase();
    let csp = head
        .lines()
        .find_map(|l| l.strip_prefix("content-security-policy: "))
        .expect("CSP header");
    for directive in [
        "default-src 'none'",
        "connect-src 'self'",
        "img-src 'self' data:",
        "frame-ancestors 'none'",
        "base-uri 'none'",
        "form-action 'none'",
    ] {
        assert!(csp.contains(directive), "CSP lacks {directive}: {csp}");
    }
    assert!(!csp.contains("unsafe-eval"));
    assert!(!csp.contains('*'), "no wildcard sources: {csp}");
    assert!(resp.contains("Multiplay Remote"));
}

#[test]
fn web_client_loads_no_third_party_resources() {
    let html = MOBILE_WEB_CLIENT_HTML.to_lowercase();
    for needle in [
        "http://",
        "https://",
        "src=",
        "<link",
        "@import",
        "url(",
        "//cdn",
        "xmlhttprequest",
        "eval(",
        "new function",
    ] {
        assert!(!html.contains(needle), "web client contains {needle:?}");
    }
}

#[test]
fn web_client_only_fetches_its_own_upload_endpoint() {
    let fetches = MOBILE_WEB_CLIENT_HTML.matches("fetch(").count();
    let uploads = MOBILE_WEB_CLIENT_HTML
        .matches("fetch('/api/upload'")
        .count();
    assert_eq!(
        fetches, uploads,
        "every fetch must go to the host's own upload endpoint"
    );
    assert!(
        MOBILE_WEB_CLIENT_HTML.contains("'Uploads'"),
        "upload button follows the host switch"
    );
}

#[test]
fn web_client_never_renders_host_text_as_markup() {
    for sink in [
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
    ] {
        assert!(
            !MOBILE_WEB_CLIENT_HTML.contains(sink),
            "track titles are attacker-controlled; {sink} is not allowed"
        );
    }
    assert!(MOBILE_WEB_CLIENT_HTML.contains("textContent"));
}

#[test]
fn web_client_speaks_protocol_v2_requests() {
    for needle in [
        "v: 2",
        "'RequestTrack'",
        "'Search'",
        "'RequestUpdate'",
        "'RequestRefused'",
        "'SearchResults'",
        "kind: 'Link'",
        "kind: 'Search'",
        "Pending",
        "Accepted",
        "Rejected",
        "Playing",
    ] {
        assert!(
            MOBILE_WEB_CLIENT_HTML.contains(needle),
            "web client lacks {needle:?}"
        );
    }
    // Mobile-friendly: viewport set, inputs big enough that iOS does not zoom on focus.
    assert!(MOBILE_WEB_CLIENT_HTML.contains("name=\"viewport\""));
    assert!(MOBILE_WEB_CLIENT_HTML.contains("font-size: 16px"));
    assert!(!MOBILE_WEB_CLIENT_HTML.contains("user-scalable=no"));
}
