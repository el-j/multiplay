//! Containing panics from code that parses untrusted data (TASK-185): a third-party decoder that
//! panics on a damaged file must fail that file, not take the playback thread down with it.

#![forbid(unsafe_code)]

use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};

thread_local! {
    static CONTAINED: Cell<u32> = const { Cell::new(0) };
}

/// Runs `f`; a panic inside it becomes `Err` with the panic message. Only for code whose state
/// is thrown away after a failure (one damaged file), never for shared state.
///
/// # Errors
/// The panic's message when `f` panicked.
pub fn contain_panic<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    CONTAINED.with(|c| c.set(c.get() + 1));
    let result = catch_unwind(AssertUnwindSafe(f));
    CONTAINED.with(|c| c.set(c.get().saturating_sub(1)));
    result.map_err(|payload| {
        payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string())
    })
}

/// True on a thread that is inside [`contain_panic`]: the crash hook logs such panics instead of
/// writing a crash report.
#[must_use]
pub fn panic_is_contained() -> bool {
    CONTAINED.with(|c| c.get() > 0)
}
