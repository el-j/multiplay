//! Authentic Winamp 2.x bitmap texture decoder and egui texture cache.
//! Pure-Rust BMP decompression and sub-rectangle sprite extraction.

#![forbid(unsafe_code)]

use eframe::egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::HashMap;

/// Cache of GPU textures extracted from Winamp .wsz bitmaps.
#[derive(Default)]
pub struct SkinTextureCache {
    textures: HashMap<String, TextureHandle>,
}

impl SkinTextureCache {
    /// Loads or retrieves a cached texture for a skin bitmap (e.g. "main.bmp").
    pub fn get_or_load(
        &mut self,
        ctx: &Context,
        bitmap_name: &str,
        bmp_bytes: &[u8],
    ) -> Option<TextureHandle> {
        let key = bitmap_name.to_ascii_lowercase();
        if let Some(handle) = self.textures.get(&key) {
            return Some(handle.clone());
        }

        let img = image::load_from_memory(bmp_bytes).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let pixels = img.as_raw();

        let mut color_pixels = Vec::with_capacity(size[0] * size[1]);
        for chunk in pixels.as_chunks::<4>().0 {
            color_pixels.push(Color32::from_rgba_unmultiplied(
                chunk[0], chunk[1], chunk[2], chunk[3],
            ));
        }

        let color_image = ColorImage {
            size,
            pixels: color_pixels,
        };

        let handle = ctx.load_texture(
            format!("winamp_skin_{key}"),
            color_image,
            TextureOptions::NEAREST,
        );

        self.textures.insert(key, handle.clone());
        Some(handle)
    }

    /// Clears cached textures when hot-reloading a new skin.
    pub fn clear(&mut self) {
        self.textures.clear();
    }

    /// Returns the number of currently loaded textures.
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Returns true if no textures are loaded.
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skin_texture_cache_lifecycle() {
        let mut cache = SkinTextureCache::default();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        cache.clear();
        assert!(cache.is_empty());
    }
}
