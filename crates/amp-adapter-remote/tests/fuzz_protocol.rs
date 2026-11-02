//! Whatever a phone sends (TASK-185): arbitrary or damaged messages are refused or understood,
//! never a panic.

#![forbid(unsafe_code)]

use amp_adapter_remote::parse_client_message;
use proptest::prelude::*;

const VALID: [&str; 4] = [
    r#"{"type":"Play"}"#,
    r#"{"type":"SetVolume","volume":0.5}"#,
    r#"{"type":"RequestTrack","reference":{"Link":"https://youtu.be/dQw4w9WgXcQ"}}"#,
    r#"{"type":"Search","query":"daft punk"}"#,
];

/// Cases per property: the default in CI, many more in the nightly fuzz job (`FUZZ_CASES`).
fn cases(default: u32) -> u32 {
    std::env::var("FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(1024)))]

    #[test]
    fn arbitrary_text_never_panics(text in "\\PC{0,2000}") {
        let _ = parse_client_message(&text);
    }

    #[test]
    fn damaged_messages_never_panic(which in 0usize..VALID.len(), cut in any::<prop::sample::Index>(), insert in "\\PC{0,20}") {
        let msg = VALID[which];
        let mut at = cut.index(msg.len() + 1);
        while !msg.is_char_boundary(at) {
            at -= 1;
        }
        let damaged = format!("{}{}{}", &msg[..at], insert, &msg[at..]);
        let _ = parse_client_message(&damaged);
        let _ = parse_client_message(&msg[..at]);
    }

    #[test]
    fn deeply_nested_json_is_refused_not_overflowed(depth in 1usize..5_000) {
        let text = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        prop_assert!(parse_client_message(&text).is_err());
    }
}
