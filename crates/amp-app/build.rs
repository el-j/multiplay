//! Exports the release version and commit for `--version`.

#![forbid(unsafe_code)]

#[path = "../../build-support/build_info.rs"]
mod build_info;

fn main() {
    build_info::export();
}
