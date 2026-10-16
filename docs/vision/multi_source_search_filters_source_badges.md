# Multi-Source Search Aggregator, Source Toggles & Icon Badges

This document specifies the filtered multi-source search query pipeline, source-exclusion engine, and UI visual indicators (icons/badges) for search results and playlists.

---

## 1. Domain Types for Multi-Source Filtering & Attribution

Every source provider is tagged with a distinct identity, display glyph, and filter mask.

```rust
// crates/amp-domain/src/source_badge.rs
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceKind {
    LocalFile,
    NetworkSmb,
    YouTube,
    Spotify,
    Bandcamp,
    SoundCloud,
    Soulseek,
    P2PShared,
}

impl SourceKind {
    /// Distinct Unicode / ASCII indicator for terminal and GUI renderers
    pub fn badge_icon(&self) -> &'static str {
        match self {
            Self::LocalFile  => "󰉋 [LOC]", // Folder icon
            Self::NetworkSmb => "󰒍 [SMB]", // Network drive
            Self::YouTube    => " [YT]",  // YouTube
            Self::Spotify    => " [SPOT]",// Spotify
            Self::Bandcamp   => "󰓇 [BC]",  // Bandcamp
            Self::SoundCloud => " [SC]",  // SoundCloud
            Self::Soulseek   => "󰛳 [SLSK]",// Soulseek P2P
            Self::P2PShared  => "󱘖 [P2P]", // Encrypted Private Mesh
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::LocalFile  => "Local Library",
            Self::NetworkSmb => "SMB / Network Share",
            Self::YouTube    => "YouTube Audio",
            Self::Spotify    => "Spotify Premium",
            Self::Bandcamp   => "Bandcamp",
            Self::SoundCloud => "SoundCloud",
            Self::Soulseek   => "Soulseek Network",
            Self::P2PShared  => "Direct P2P Mesh",
        }
    }
}

/// Bitmask or active set for filtering search queries dynamically
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFilterMask {
    pub enabled_sources: std::collections::HashSet<SourceKind>,
}

impl Default for SourceFilterMask {
    fn default() -> Self {
        use SourceKind::*;
        let mut enabled = std::collections::HashSet::new();
        enabled.insert(LocalFile);
        enabled.insert(NetworkSmb);
        enabled.insert(YouTube);
        enabled.insert(Spotify);
        enabled.insert(Bandcamp);
        enabled.insert(SoundCloud);
        enabled.insert(Soulseek);
        enabled.insert(P2PShared);
        Self { enabled_sources: enabled }
    }
}

impl SourceFilterMask {
    pub fn is_enabled(&self, kind: SourceKind) -> bool {
        self.enabled_sources.contains(&kind)
    }

    pub fn toggle(&mut self, kind: SourceKind) {
        if self.enabled_sources.contains(&kind) {
            self.enabled_sources.remove(&kind);
        } else {
            self.enabled_sources.insert(kind);
        }
    }
}
```

---

## 2. Updated Search Port with Exclusion Criteria

```rust
// crates/amp-ports/src/driving.rs (Extensions)
use crate::driven::ProviderSearchPort;
use amp_domain::source_badge::{SourceFilterMask, SourceKind};
use amp_domain::Track;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SearchQueryOptions {
    pub text: String,
    pub filter_mask: SourceFilterMask,
    pub timeout_millis: u64,
}

#[async_trait]
pub trait FilteredSearchPort: Send + Sync {
    async fn search(&self, options: SearchQueryOptions) -> Vec<Track>;
}
```

---

## 3. Parallel Search Coordinator with Source Exclusion

```rust
// crates/amp-core-engine/src/filtered_search.rs
use amp_domain::source_badge::{SourceFilterMask, SourceKind};
use amp_domain::Track;
use amp_ports::driven::ProviderSearchPort;
use amp_ports::driving::{FilteredSearchPort, SearchQueryOptions};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

pub struct FilteredSearchCoordinator {
    providers: Vec<(SourceKind, Arc<dyn ProviderSearchPort>)>,
}

impl FilteredSearchCoordinator {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register_provider(
        &mut self,
        kind: SourceKind,
        provider: Arc<dyn ProviderSearchPort>,
    ) {
        self.providers.push((kind, provider));
    }
}

#[async_trait]
impl FilteredSearchPort for FilteredSearchCoordinator {
    async fn search(&self, options: SearchQueryOptions) -> Vec<Track> {
        let mut join_set = JoinSet::new();

        // Spawn search tasks ONLY for non-excluded sources
        for (kind, provider) in &self.providers {
            if !options.filter_mask.is_enabled(*kind) {
                continue; // Skip excluded provider
            }

            let p = Arc::clone(provider);
            let q = options.text.clone();
            let k = *kind;

            join_set.spawn(async move {
                let results = p.query(&q).await.unwrap_or_default();
                (k, results)
            });
        }

        let mut aggregated = Vec::new();
        let timeout = tokio::time::sleep(Duration::from_millis(options.timeout_millis));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                next_res = join_set.join_next() => {
                    match next_res {
                        Some(Ok((_kind, tracks))) => {
                            aggregated.extend(tracks);
                        }
                        Some(Err(e)) => eprintln!("Search worker task panicked: {e}"),
                        None => break, // All tasks finished
                    }
                }
                _ = &mut timeout => {
                    join_set.abort_all();
                    break;
                }
            }
        }

        aggregated
    }
}
```

---

## 4. Ratatui TUI Search Dialog & Playlist with Source Badges

This UI widget displays real-time toggleable source filters, search results with provider icons, and a playlist displaying the source badge on each item.

```rust
// crates/amp-adapter-tui/src/search_view.rs
use amp_domain::source_badge::{SourceFilterMask, SourceKind};
use amp_domain::{AudioSource, Track};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub struct SearchViewState {
    pub search_input: String,
    pub filter_mask: SourceFilterMask,
    pub search_results: Vec<Track>,
    pub selected_result_idx: usize,
    pub active_filter_idx: usize,
}

pub fn render_search_modal(f: &mut Frame, state: &SearchViewState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input Omnibar
            Constraint::Length(3), // Source Filter Pills
            Constraint::Min(8),    // Results list
        ])
        .split(area);

    // 1. Omnibar
    let search_bar = Paragraph::new(state.search_input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 󰍉 Search All Audio Sources (Press Tab to toggle sources) "),
        );
    f.render_widget(search_bar, chunks[0]);

    // 2. Filter Pills
    let all_sources = [
        SourceKind::LocalFile,
        SourceKind::NetworkSmb,
        SourceKind::YouTube,
        SourceKind::Spotify,
        SourceKind::Bandcamp,
        SourceKind::SoundCloud,
        SourceKind::Soulseek,
        SourceKind::P2PShared,
    ];

    let mut filter_spans = Vec::new();
    for (i, kind) in all_sources.iter().enumerate() {
        let enabled = state.filter_mask.is_enabled(*kind);
        let is_active_cursor = i == state.active_filter_idx;

        let style = match (enabled, is_active_cursor) {
            (true, true) => Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
            (true, false) => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            (false, true) => Style::default().fg(Color::Black).bg(Color::DarkGray),
            (false, false) => Style::default().fg(Color::DarkGray),
        };

        let label = format!(" [{}] {} ", if enabled { "✔" } else { "✘" }, kind.display_label());
        filter_spans.push(Span::styled(label, style));
        filter_spans.push(Span::raw(" "));
    }

    let filter_row = Paragraph::new(Line::from(filter_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active Source Filters "),
    );
    f.render_widget(filter_row, chunks[1]);

    // 3. Search Results List with Source Badges
    let items: Vec<ListItem> = state
        .search_results
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let kind = map_source_to_kind(&track.source);
            let badge_str = kind.badge_icon();

            let badge_color = match kind {
                SourceKind::LocalFile => Color::Cyan,
                SourceKind::NetworkSmb => Color::Blue,
                SourceKind::YouTube => Color::Red,
                SourceKind::Spotify => Color::Green,
                SourceKind::Bandcamp => Color::LightCyan,
                SourceKind::SoundCloud => Color::Rgb(255, 140, 0),
                SourceKind::Soulseek => Color::Magenta,
                SourceKind::P2PShared => Color::Yellow,
            };

            let duration_sec = track.metadata.duration.map(|d| d.as_secs()).unwrap_or(0);
            let time_str = format!("{:02}:{:02}", duration_sec / 60, duration_sec % 60);

            let is_selected = i == state.selected_result_idx;
            let row_style = if is_selected {
                Style::default().bg(Color::Rgb(40, 40, 40)).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!(" {badge_str} "), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<28}", truncate(&track.metadata.artist, 28)), Style::default().fg(Color::White)),
                Span::raw(" - "),
                Span::styled(format!("{:<35}", truncate(&track.metadata.title, 35)), Style::default().fg(Color::Gray)),
                Span::styled(format!(" {:>6} ", time_str), Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line).style(row_style)
        })
        .collect();

    let results_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Results (Enter: Add to Playlist | 'd': Direct Download) "),
    );
    f.render_widget(results_list, chunks[2]);
}

fn map_source_to_kind(source: &AudioSource) -> SourceKind {
    match source {
        AudioSource::LocalFile { .. } => SourceKind::LocalFile,
        AudioSource::NetworkSmb { .. } => SourceKind::NetworkSmb,
        AudioSource::DirectHttp { .. } => SourceKind::Bandcamp,
        AudioSource::YouTube { .. } => SourceKind::YouTube,
        AudioSource::Spotify { .. } => SourceKind::Spotify,
        AudioSource::Bandcamp { .. } => SourceKind::Bandcamp,
        AudioSource::SoundCloud { .. } => SourceKind::SoundCloud,
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    } else {
        s.to_string()
    }
}
```