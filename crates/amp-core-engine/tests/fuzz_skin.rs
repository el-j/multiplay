//! `.wsz` skins come from anywhere (TASK-185): arbitrary or damaged bytes never panic, and a zip
//! bomb or an entry flood is refused before it can use much memory.

#![forbid(unsafe_code)]

use amp_core_engine::skin::{SkinLoader, MAX_SKIN_ENTRIES, MAX_SKIN_ENTRY_BYTES};
use proptest::prelude::*;
use std::io::Write;

fn zip_with(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    out.into_inner()
}

fn small_skin() -> Vec<u8> {
    zip_with(&[
        ("main.bmp", vec![0x42; 2_000]),
        ("viscolor.txt", b"0,0,0\n24,33,41\n".to_vec()),
        ("pledit.txt", b"[Text]\nNormal=#00FF00\n".to_vec()),
    ])
}

#[test]
fn a_zip_bomb_is_refused() {
    // 64 MiB of zeros deflates to about 64 KiB.
    let bomb = zip_with(&[("main.bmp", vec![0; 64 << 20])]);
    assert!(bomb.len() < 1 << 20, "{}", bomb.len());
    let Err(err) = SkinLoader::load_skin_from_wsz_bytes(&bomb, "bomb") else {
        panic!("refused");
    };
    assert!(err.contains("too large"), "{err}");
    const { assert!(MAX_SKIN_ENTRY_BYTES < 64 << 20) };
}

#[test]
fn an_entry_flood_is_refused() {
    let names: Vec<String> = (0..MAX_SKIN_ENTRIES + 1)
        .map(|i| format!("f{i}.txt"))
        .collect();
    let entries: Vec<(&str, Vec<u8>)> = names.iter().map(|n| (n.as_str(), Vec::new())).collect();
    let Err(err) = SkinLoader::load_skin_from_wsz_bytes(&zip_with(&entries), "flood") else {
        panic!("refused");
    };
    assert!(err.contains("too many"), "{err}");
}

/// Found by the fuzzer: six bytes of non-ASCII text split inside a character.
#[test]
fn non_ascii_colour_values_are_ignored() {
    let mut theme = amp_core_engine::WinampTheme::default();
    let before = theme.pledit_fg;
    SkinLoader::parse_pledit("[Text]\nNormal=ଓᾶ\nNormalBG=#ZZZZZZ", &mut theme);
    assert_eq!(theme.pledit_fg, before);
}

#[test]
fn a_real_skin_still_loads() {
    let skin = SkinLoader::load_skin_from_wsz_bytes(&small_skin(), "ok").expect("loads");
    assert!(skin.bitmaps.contains_key("main.bmp"));
}

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
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = SkinLoader::load_skin_from_wsz_bytes(&bytes, "fuzz");
    }

    #[test]
    fn damaged_skins_never_panic(flips in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..32)) {
        let mut bytes = small_skin();
        for (at, value) in flips {
            let i = at.index(bytes.len());
            bytes[i] = value;
        }
        let _ = SkinLoader::load_skin_from_wsz_bytes(&bytes, "fuzz");
    }

    #[test]
    fn arbitrary_skin_text_never_panics(text in "\\PC{0,2000}") {
        let mut theme = amp_core_engine::WinampTheme::default();
        SkinLoader::parse_viscolor(&text, &mut theme);
        SkinLoader::parse_pledit(&text, &mut theme);
    }
}
