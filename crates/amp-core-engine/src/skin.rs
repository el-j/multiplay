//! Winamp Classic Skin (.wsz) Decompressor and Palette Extractor.
//! Parses in-memory ZIP archives, viscolor.txt, and pledit.txt into a typed WinampTheme.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

pub type RgbColor = (u8, u8, u8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinampTheme {
    pub name: String,
    pub bg_color: RgbColor,
    pub fg_color: RgbColor,
    pub accent_color: RgbColor,
    pub vfd_bg: RgbColor,
    pub vfd_fg: RgbColor,
    pub viscolor_gradient: [RgbColor; 16],
    pub viscolor_bg: RgbColor,
    pub viscolor_dots: RgbColor,
    pub viscolor_osc: [RgbColor; 5],
    pub pledit_bg: RgbColor,
    pub pledit_fg: RgbColor,
}

impl Default for WinampTheme {
    fn default() -> Self {
        Self {
            name: "multiplay Classic".to_string(),
            bg_color: (24, 24, 27),
            fg_color: (212, 212, 216),
            accent_color: (250, 204, 21),
            vfd_bg: (9, 24, 18),
            vfd_fg: (52, 211, 153),
            viscolor_gradient: [
                (255, 0, 0), // Peak
                (255, 60, 0),
                (255, 120, 0),
                (255, 180, 0),
                (255, 230, 0),
                (230, 255, 0),
                (180, 255, 0),
                (120, 255, 0),
                (60, 255, 0),
                (0, 255, 50),
                (0, 255, 120),
                (0, 255, 200),
                (0, 220, 255),
                (0, 160, 255),
                (0, 100, 255),
                (0, 50, 255), // Floor
            ],
            viscolor_bg: (0, 0, 0),
            viscolor_dots: (240, 255, 255),
            viscolor_osc: [
                (0, 255, 0),
                (50, 255, 50),
                (100, 255, 100),
                (150, 255, 150),
                (200, 255, 200),
            ],
            pledit_bg: (0, 0, 0),
            pledit_fg: (0, 255, 0),
        }
    }
}

/// Fully extracted Winamp classic skin archive with theme and raw bitmap assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinampSkin {
    pub theme: WinampTheme,
    pub bitmaps: HashMap<String, Vec<u8>>,
}

pub struct SkinLoader;

impl SkinLoader {
    /// Loads complete Winamp skin from in-memory .wsz zip bytes.
    pub fn load_skin_from_wsz_bytes(bytes: &[u8], skin_name: &str) -> Result<WinampSkin, String> {
        let cursor = Cursor::new(bytes);
        let mut archive =
            ZipArchive::new(cursor).map_err(|e| format!("Failed to open .wsz zip: {e}"))?;

        let mut theme = WinampTheme {
            name: skin_name.to_string(),
            ..Default::default()
        };
        let mut bitmaps = HashMap::new();
        let mut viscolor_raw = None;
        let mut pledit_raw = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let raw_name = file.name().map_err(|e| e.to_string())?.to_ascii_lowercase();
            let base_name = Path::new(&raw_name)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&raw_name)
                .to_string();

            if base_name == "viscolor.txt" {
                let mut content = String::new();
                let _ = file.read_to_string(&mut content);
                viscolor_raw = Some(content);
            } else if base_name == "pledit.txt" {
                let mut content = String::new();
                let _ = file.read_to_string(&mut content);
                pledit_raw = Some(content);
            } else if base_name.ends_with(".bmp") {
                let mut buf = Vec::new();
                let _ = file.read_to_end(&mut buf);
                bitmaps.insert(base_name, buf);
            }
        }

        if let Some(content) = viscolor_raw {
            Self::parse_viscolor(&content, &mut theme);
        }

        if let Some(content) = pledit_raw {
            Self::parse_pledit(&content, &mut theme);
        }

        Ok(WinampSkin { theme, bitmaps })
    }

    /// Loads complete Winamp skin from .wsz file on disk.
    pub fn load_skin_from_wsz_file(path: &Path) -> Result<WinampSkin, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read .wsz: {e}"))?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Custom Skin");
        Self::load_skin_from_wsz_bytes(&bytes, name)
    }

    /// Loads and parses skin theme from in-memory .wsz zip bytes.
    pub fn load_from_wsz_bytes(bytes: &[u8], skin_name: &str) -> Result<WinampTheme, String> {
        Self::load_skin_from_wsz_bytes(bytes, skin_name).map(|s| s.theme)
    }

    /// Parses viscolor.txt palette (background, dots, 16 spectrum bands, oscilloscope lines).
    pub fn parse_viscolor(content: &str, theme: &mut WinampTheme) {
        let lines: Vec<&str> = content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('/') && !l.starts_with(';'))
            .collect();

        if lines.is_empty() {
            return;
        }

        if let Some(bg) = Self::parse_rgb_line(lines[0]) {
            theme.viscolor_bg = bg;
        }
        if lines.len() > 1 {
            if let Some(dots) = Self::parse_rgb_line(lines[1]) {
                theme.viscolor_dots = dots;
            }
        }
        for i in 0..16 {
            if lines.len() > i + 2 {
                if let Some(col) = Self::parse_rgb_line(lines[i + 2]) {
                    theme.viscolor_gradient[i] = col;
                }
            }
        }
        for i in 0..5 {
            if lines.len() > i + 18 {
                if let Some(col) = Self::parse_rgb_line(lines[i + 18]) {
                    theme.viscolor_osc[i] = col;
                }
            }
        }
    }

    fn parse_rgb_line(line: &str) -> Option<RgbColor> {
        let parts: Vec<&str> = line
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            Some((r, g, b))
        } else {
            None
        }
    }

    /// Parses pledit.txt playlist colors (NormalBG, Normal, Current, etc.)
    pub fn parse_pledit(content: &str, theme: &mut WinampTheme) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some((key, val)) = trimmed.split_once('=') {
                let k = key.trim().to_ascii_lowercase();
                let v = val.trim();
                if let Some(rgb) = Self::parse_hex_color(v) {
                    match k.as_str() {
                        "normalbg" => theme.pledit_bg = rgb,
                        "normal" => theme.pledit_fg = rgb,
                        _ => {}
                    }
                }
            }
        }
    }

    fn parse_hex_color(val: &str) -> Option<RgbColor> {
        let clean = val.trim_start_matches('#');
        if clean.len() == 6 {
            let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
            let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
            let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
            Some((r, g, b))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    #[test]
    fn test_skin_loader_default_theme() {
        let theme = WinampTheme::default();
        assert_eq!(theme.name, "multiplay Classic");
        assert_eq!(theme.viscolor_gradient[0], (255, 0, 0));
        assert_eq!(theme.pledit_bg, (0, 0, 0));
        assert_eq!(theme.pledit_fg, (0, 255, 0));
    }

    #[test]
    fn test_skin_loader_from_wsz_zip() {
        // Construct an in-memory ZIP archive with viscolor.txt and pledit.txt
        let mut zip_buffer = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut zip_buffer));
            let options = SimpleFileOptions::default();

            // Write viscolor.txt
            writer.start_file("viscolor.txt", options).unwrap();
            let mut vis_content = String::new();
            vis_content.push_str("0,0,0\n0,0,0\n"); // lines 0, 1
            for i in 0..16 {
                vis_content.push_str(&format!("{},{},{}\n", i * 15, i * 10, i * 5));
            }
            writer.write_all(vis_content.as_bytes()).unwrap();

            // Write pledit.txt
            writer.start_file("pledit.txt", options).unwrap();
            let pledit_content = "[Text]\nNormalBG=#1A1A2E\nNormal=#E94560\n";
            writer.write_all(pledit_content.as_bytes()).unwrap();

            writer.finish().unwrap();
        }

        let theme = SkinLoader::load_from_wsz_bytes(&zip_buffer, "Neon 98").unwrap();
        assert_eq!(theme.name, "Neon 98");
        assert_eq!(theme.viscolor_gradient[0], (0, 0, 0));
        assert_eq!(theme.viscolor_gradient[15], (225, 150, 75));
        assert_eq!(theme.pledit_bg, (0x1A, 0x1A, 0x2E));
        assert_eq!(theme.pledit_fg, (0xE9, 0x45, 0x60));
    }
}
