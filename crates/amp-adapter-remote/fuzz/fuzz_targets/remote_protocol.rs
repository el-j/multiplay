//! Fuzzes the remote client message decoder: it must never panic on any input.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = amp_adapter_remote::parse_client_message(text);
    }
});
