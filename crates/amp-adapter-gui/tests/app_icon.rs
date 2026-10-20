//! The app icon: the window icon decodes, the committed icon files are current and well formed,
//! and the container writers (macOS .icns, Windows .ico and .res) produce what the OS expects.

#![forbid(unsafe_code)]

use amp_adapter_gui::app_icon::{self, formats};
use std::path::PathBuf;

fn assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icon")
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

#[test]
fn window_icon_is_a_square_icon_with_transparent_corners() {
    let icon = app_icon::window_icon();
    assert_eq!(icon.width, icon.height);
    assert!(icon.width >= 64);
    assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    let alpha = |x: u32, y: u32| icon.rgba[((y * icon.width + x) * 4 + 3) as usize];
    assert_eq!(alpha(0, 0), 0, "corners are transparent");
    let c = icon.width / 2;
    assert_eq!(alpha(c, c), 255, "the middle is opaque");
}

#[test]
fn every_size_is_committed_with_its_real_dimensions() {
    for size in app_icon::SIZES {
        let path = assets().join(format!("multiplay-{size}.png"));
        let img = image::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            (img.width(), img.height()),
            (size, size),
            "{}",
            path.display()
        );
    }
    for name in ["multiplay.icns", "multiplay.ico"] {
        assert!(assets().join(name).is_file(), "{name} is missing");
    }
}

#[test]
fn ico_directory_points_at_each_png() {
    let a = vec![1u8; 10];
    let mut png16 = PNG.to_vec();
    png16.extend_from_slice(&a);
    let png256 = [PNG, b"big"].concat();
    let ico = formats::ico(&[(16, &png16), (256, &png256)]);
    assert_eq!(
        (u16_at(&ico, 0), u16_at(&ico, 2), u16_at(&ico, 4)),
        (0, 1, 2)
    );
    // 256 px is written as 0 in the one-byte width and height fields.
    assert_eq!((ico[6], ico[7], ico[22], ico[23]), (16, 16, 0, 0));
    for (entry, png) in [(6usize, &png16[..]), (22, &png256[..])] {
        let len = u32_at(&ico, entry + 8) as usize;
        let off = u32_at(&ico, entry + 12) as usize;
        assert_eq!(&ico[off..off + len], png);
    }
}

#[test]
fn icns_lists_typed_chunks_with_lengths() {
    let png = [PNG, b"x"].concat();
    let icns = formats::icns(&[(*b"icp4", &png), (*b"ic10", &png)]);
    assert_eq!(&icns[..4], b"icns");
    assert_eq!(
        u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
        icns.len()
    );
    assert_eq!(&icns[8..12], b"icp4");
    let first = u32::from_be_bytes(icns[12..16].try_into().unwrap()) as usize;
    assert_eq!(first, 8 + png.len());
    assert_eq!(&icns[8 + first..12 + first], b"ic10");
}

#[test]
fn res_holds_icons_and_one_group_icon() {
    let png = [PNG, b"res"].concat();
    let res = formats::res(&[(16, &png), (32, &png)]);
    // The empty resource every .res file starts with.
    assert_eq!(&res[..8], &[0, 0, 0, 0, 0x20, 0, 0, 0]);
    let mut at = 32;
    let mut kinds = Vec::new();
    while at < res.len() {
        let data = u32_at(&res, at) as usize;
        let header = u32_at(&res, at + 4) as usize;
        assert_eq!(u16_at(&res, at + 8), 0xFFFF, "type is an ordinal");
        kinds.push(u16_at(&res, at + 10));
        at += header + data;
        at = (at + 3) & !3;
    }
    assert_eq!(at, res.len());
    assert_eq!(kinds, vec![3, 3, 14], "two RT_ICON then RT_GROUP_ICON");
}

#[test]
fn committed_containers_match_the_committed_pngs() {
    let read = |n: &str| std::fs::read(assets().join(n)).unwrap();
    assert_eq!(
        read("multiplay.ico"),
        app_icon::build_ico(&assets()).unwrap()
    );
    assert_eq!(
        read("multiplay.icns"),
        app_icon::build_icns(&assets()).unwrap()
    );
}

#[test]
fn every_window_carries_the_same_decoded_icon() {
    use amp_adapter_gui::chrome::classic_viewport;
    let a = classic_viewport("main", false).icon.expect("icon set");
    let b = classic_viewport("eq", true).icon.expect("icon set");
    assert!(
        std::sync::Arc::ptr_eq(&a, &b),
        "decoded once, not per frame"
    );
    assert!(a.width >= 64);
}
