//! Every icon the GUI draws must exist in the bundled fonts. egui 0.37 moved its emoji and icon
//! fonts behind the `monochrome_emoji_fonts` feature; without it every icon is a hollow box.
//! The icons are collected from the source, so new ones are covered automatically.

#![forbid(unsafe_code)]

use eframe::egui::{self, FontFamily, FontId, RawInput};
use std::collections::BTreeSet;
use std::path::Path;

/// Symbol characters (arrows, shapes, emoji...) used outside comments in `dir`.
fn icons_in(dir: &Path, out: &mut BTreeSet<char>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            icons_in(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
                out.extend(
                    line.chars()
                        .filter(|c| *c == '•' || (*c >= '\u{2190}' && !matches!(c, '“' | '”'))),
                );
            }
        }
    }
}

#[test]
fn every_icon_used_by_the_gui_has_a_glyph() {
    let mut icons = BTreeSet::new();
    icons_in(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut icons,
    );
    assert!(
        icons.contains(&'▶') && icons.contains(&'📁'),
        "scan found {icons:?}"
    );

    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(RawInput::default(), |_| {});
    output.textures_delta.clear();
    let missing: String = ctx.fonts_mut(|fonts| {
        let mut missing = String::new();
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let font = FontId::new(12.0, family);
            for &c in &icons {
                if !fonts.has_glyph(&font, c) && !missing.contains(c) {
                    missing.push(c);
                }
            }
        }
        missing
    });
    assert!(
        missing.is_empty(),
        "icons without a glyph (drawn as boxes): {missing}"
    );
}
