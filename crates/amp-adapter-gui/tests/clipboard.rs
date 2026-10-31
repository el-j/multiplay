//! Copy and paste need eframe's `clipboard` feature. The GUI turns eframe's default features off,
//! so the feature has to be listed explicitly; without it Cmd/Ctrl+V does nothing anywhere
//! (pasting links into search was broken that way).

#![forbid(unsafe_code)]

#[test]
fn eframe_is_built_with_clipboard_support() {
    let manifest = include_str!("../Cargo.toml");
    let eframe = manifest
        .lines()
        .find(|l| l.trim_start().starts_with("eframe ="))
        .expect("eframe dependency");
    assert!(eframe.contains("\"clipboard\""), "{eframe}");
}
