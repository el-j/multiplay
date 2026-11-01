//! Bandcamp's answers are parsed defensively (TASK-185): arbitrary or damaged JSON is an error
//! or results, never a panic.

#![forbid(unsafe_code)]

use amp_adapter_bandcamp::parse_search_response;
use proptest::prelude::*;

const FIXTURE: &str = include_str!("fixtures/autocomplete_elastic.json");

/// Cases per property: the default in CI, many more in the nightly fuzz job (`FUZZ_CASES`).
fn cases(default: u32) -> u32 {
    std::env::var("FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(512)))]

    #[test]
    fn arbitrary_text_never_panics(text in "\\PC{0,2000}") {
        let _ = parse_search_response(&text);
    }

    #[test]
    fn damaged_responses_never_panic(cut in any::<prop::sample::Index>(), insert in "\\PC{0,12}") {
        let mut at = cut.index(FIXTURE.len() + 1);
        while !FIXTURE.is_char_boundary(at) {
            at -= 1;
        }
        let _ = parse_search_response(&format!("{}{}{}", &FIXTURE[..at], insert, &FIXTURE[at..]));
        let _ = parse_search_response(&FIXTURE[..at]);
    }
}
