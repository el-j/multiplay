//! QR code rendering of the pairing URL for terminal and GUI display.
//! Pairing tokens themselves live in [`crate::auth`].

#![forbid(unsafe_code)]

use qrcode::QrCode;

/// Pure-Rust QR Code renderer for terminal ASCII and GUI bitmap matrix.
#[derive(Debug, Clone, Copy)]
pub struct QrPairingRenderer;

impl QrPairingRenderer {
    /// Renders an ASCII text block representing the QR code for terminal display.
    pub fn render_ascii(url: &str) -> Result<String, String> {
        let code = QrCode::new(url.as_bytes()).map_err(|e| e.to_string())?;
        let rendered = code
            .render::<qrcode::render::unicode::Dense1x2>()
            .dark_color(qrcode::render::unicode::Dense1x2::Light)
            .light_color(qrcode::render::unicode::Dense1x2::Dark)
            .build();
        Ok(rendered)
    }

    /// Renders a 2D boolean module matrix (true = dark, false = light) for GUI canvas widgets.
    pub fn render_matrix(url: &str) -> Result<Vec<Vec<bool>>, String> {
        let code = QrCode::new(url.as_bytes()).map_err(|e| e.to_string())?;
        let width = code.width();
        let colors = code.into_colors();

        let mut matrix = Vec::with_capacity(width);
        for row_chunk in colors.chunks(width) {
            let row = row_chunk
                .iter()
                .map(|color| *color == qrcode::Color::Dark)
                .collect();
            matrix.push(row);
        }

        Ok(matrix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_qr_matrix_generation() {
        let url = "http://192.168.1.100:9002/?token=ABC12345ABC12345";
        let matrix = QrPairingRenderer::render_matrix(url).expect("matrix render failed");
        assert!(!matrix.is_empty());
        assert_eq!(matrix.len(), matrix[0].len());

        let ascii = QrPairingRenderer::render_ascii(url).expect("ascii render failed");
        assert!(!ascii.is_empty());
    }
}
