//! Ephemeral pairing token management and QR code generation for mobile clients.

#![forbid(unsafe_code)]

use qrcode::QrCode;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Active pairing token record.
#[derive(Debug, Clone)]
struct TokenRecord {
    expires_at_epoch: u64,
}

/// Ephemeral pairing token manager enforcing time-to-live expiration.
#[derive(Debug, Default)]
pub struct PairingManager {
    active_tokens: HashMap<String, TokenRecord>,
}

impl PairingManager {
    /// Creates a new empty pairing manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            active_tokens: HashMap::new(),
        }
    }

    /// Generates a new unique pairing token with the given TTL in seconds.
    pub fn generate_token(&mut self, ttl_secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 8-character uppercase hex token derived from UUID
        let token = Uuid::new_v4().simple().to_string()[..8].to_uppercase();
        self.active_tokens.insert(
            token.clone(),
            TokenRecord {
                expires_at_epoch: now + ttl_secs,
            },
        );
        token
    }

    /// Validates and consumes a pairing token.
    pub fn validate_and_consume(&mut self, token: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(record) = self.active_tokens.remove(token) {
            record.expires_at_epoch >= now
        } else {
            false
        }
    }

    /// Checks if a token is valid without consuming it.
    #[must_use]
    pub fn is_valid(&self, token: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.active_tokens
            .get(token)
            .map(|r| r.expires_at_epoch >= now)
            .unwrap_or(false)
    }
}

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
    fn test_pairing_token_lifecycle() {
        let mut manager = PairingManager::new();
        let token = manager.generate_token(60);

        assert_eq!(token.len(), 8);
        assert!(manager.is_valid(&token));

        // Consuming should succeed
        assert!(manager.validate_and_consume(&token));

        // Reusing consumed token should fail
        assert!(!manager.validate_and_consume(&token));
    }

    #[test]
    fn test_pairing_qr_matrix_generation() {
        let url = "ws://192.168.1.100:9002?token=ABC12345";
        let matrix = QrPairingRenderer::render_matrix(url).expect("matrix render failed");
        assert!(!matrix.is_empty());
        assert_eq!(matrix.len(), matrix[0].len());

        let ascii = QrPairingRenderer::render_ascii(url).expect("ascii render failed");
        assert!(!ascii.is_empty());
    }
}
