//! Fuzzes the remote client message decoder and the track-reference validator (TASK-215):
//! neither may panic on any input.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(amp_adapter_remote::RemoteAction::RequestTrack(r)) =
            amp_adapter_remote::parse_client_message(text)
        {
            let _ = r.validate();
        }
        // Raw text as a link and as a query, without the JSON envelope in front.
        let _ = amp_adapter_remote::TrackReference::Link(text.to_string()).validate();
        let _ = amp_adapter_remote::reference::validate_query(text);
    }
});
