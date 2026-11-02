#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Distinct identification for all supported audio sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceKind {
    LocalFile,
    NetworkSmb,
    DirectHttp,
    YouTube,
    Spotify,
    Bandcamp,
    SoundCloud,
    Soulseek,
    P2PShared,
}

impl SourceKind {
    /// Distinct Unicode / ASCII indicator for terminal and GUI renderers.
    pub fn badge_icon(&self) -> &'static str {
        match self {
            Self::LocalFile => "󰉋 [LOC]",
            Self::NetworkSmb => "󰒍 [SMB]",
            Self::DirectHttp => "󰌹 [HTTP]",
            Self::YouTube => " [YT]",
            Self::Spotify => " [SPOT]",
            Self::Bandcamp => "󰓇 [BC]",
            Self::SoundCloud => " [SC]",
            Self::Soulseek => "󰛳 [SLSK]",
            Self::P2PShared => "󱘖 [P2P]",
        }
    }

    /// Human-readable label for menus and tooltips.
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::LocalFile => "Local Library",
            Self::NetworkSmb => "SMB / Network Share",
            Self::DirectHttp => "Direct HTTP Stream",
            Self::YouTube => "YouTube Audio",
            Self::Spotify => "Spotify Premium",
            Self::Bandcamp => "Bandcamp",
            Self::SoundCloud => "SoundCloud",
            Self::Soulseek => "Soulseek Network",
            Self::P2PShared => "Direct P2P Mesh",
        }
    }
}

/// Bitmask or active set for filtering search queries dynamically across providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFilterMask {
    pub enabled_sources: HashSet<SourceKind>,
}

impl Default for SourceFilterMask {
    fn default() -> Self {
        let mut enabled = HashSet::new();
        enabled.insert(SourceKind::LocalFile);
        enabled.insert(SourceKind::NetworkSmb);
        enabled.insert(SourceKind::DirectHttp);
        enabled.insert(SourceKind::YouTube);
        enabled.insert(SourceKind::Bandcamp);
        enabled.insert(SourceKind::SoundCloud);
        enabled.insert(SourceKind::Soulseek);
        enabled.insert(SourceKind::P2PShared);
        Self {
            enabled_sources: enabled,
        }
    }
}

impl SourceFilterMask {
    /// Checks whether a specific source kind is enabled.
    pub fn is_enabled(&self, kind: SourceKind) -> bool {
        self.enabled_sources.contains(&kind)
    }

    /// Toggles the enabled status of a specific source kind.
    pub fn toggle(&mut self, kind: SourceKind) {
        if self.enabled_sources.contains(&kind) {
            self.enabled_sources.remove(&kind);
        } else {
            self.enabled_sources.insert(kind);
        }
    }

    /// Enables all available sources.
    pub fn enable_all(&mut self) {
        *self = Self::default();
    }

    /// Disables all sources.
    pub fn disable_all(&mut self) {
        self.enabled_sources.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_filter_mask() {
        let mask = SourceFilterMask::default();
        assert!(mask.is_enabled(SourceKind::LocalFile));
        assert!(mask.is_enabled(SourceKind::YouTube));
        assert!(!mask.is_enabled(SourceKind::Spotify), "dropped (ADR-007)");
        assert!(mask.is_enabled(SourceKind::DirectHttp));
        assert_eq!(mask.enabled_sources.len(), 8);
    }

    #[test]
    fn test_toggle_filter_mask() {
        let mut mask = SourceFilterMask::default();
        mask.toggle(SourceKind::YouTube);
        assert!(!mask.is_enabled(SourceKind::YouTube));
        mask.toggle(SourceKind::YouTube);
        assert!(mask.is_enabled(SourceKind::YouTube));
    }

    #[test]
    fn test_badge_icons_and_labels() {
        assert_eq!(SourceKind::YouTube.badge_icon(), " [YT]");
        assert_eq!(SourceKind::YouTube.display_label(), "YouTube Audio");
    }
}
