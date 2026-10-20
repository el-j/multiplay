//! Icon containers built from PNG images: macOS `.icns`, Windows `.ico` and a compiled Windows
//! resource (`.res`) that the MSVC linker embeds into the `.exe`. Every format here accepts PNG
//! data for each size, so no image codec is needed.
//!
//! Self-contained (std only): `build.rs` includes this file directly.

#![forbid(unsafe_code)]

/// Sizes put into the Windows `.ico` and `.res`.
pub const WINDOWS_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// Windows `.ico`: one directory entry per `(size, png)`.
pub fn ico(images: &[(u32, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, png) in images {
        out.extend_from_slice(&[dim(*size), dim(*size), 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

/// macOS `.icns`: big-endian chunks of `(OSType, png)`, e.g. `icp4` (16 px) or `ic10` (1024 px).
pub fn icns(chunks: &[([u8; 4], &[u8])]) -> Vec<u8> {
    let body: usize = chunks.iter().map(|(_, png)| 8 + png.len()).sum();
    let mut out = Vec::with_capacity(8 + body);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&((8 + body) as u32).to_be_bytes());
    for (kind, png) in chunks {
        out.extend_from_slice(kind);
        out.extend_from_slice(&((8 + png.len()) as u32).to_be_bytes());
        out.extend_from_slice(png);
    }
    out
}

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

/// Compiled Windows resource with one `RT_ICON` per image and the `RT_GROUP_ICON` (id 1) that
/// Explorer shows as the program icon.
pub fn res(images: &[(u32, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    // Every .res file starts with an empty resource entry.
    resource(&mut out, 0, 0, 0, &[]);
    for (i, (_, png)) in images.iter().enumerate() {
        resource(&mut out, RT_ICON, i as u16 + 1, 0x1010, png);
    }
    let mut group = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&(images.len() as u16).to_le_bytes());
    for (i, (size, png)) in images.iter().enumerate() {
        group.extend_from_slice(&[dim(*size), dim(*size), 0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&32u16.to_le_bytes());
        group.extend_from_slice(&(png.len() as u32).to_le_bytes());
        group.extend_from_slice(&(i as u16 + 1).to_le_bytes());
    }
    resource(&mut out, RT_GROUP_ICON, 1, 0x1030, &group);
    out
}

/// Icon directories store 256 px as 0.
fn dim(size: u32) -> u8 {
    if size >= 256 {
        0
    } else {
        size as u8
    }
}

fn resource(out: &mut Vec<u8>, kind: u16, name: u16, flags: u16, data: &[u8]) {
    let header: u32 = 32;
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&header.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&name.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // data version
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // language neutral
    out.extend_from_slice(&0u32.to_le_bytes()); // version
    out.extend_from_slice(&0u32.to_le_bytes()); // characteristics
    out.extend_from_slice(data);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}
