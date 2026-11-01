//! The library browser's model (TASK-172): artists, albums, sortable and filterable tracks,
//! recently added and per-source counts, built once from the stored tracks. Sort keys are
//! lower-cased at build time so sorting 100k tracks allocates nothing per comparison.

#![forbid(unsafe_code)]

use amp_domain::{SourceKind, Track};
use std::collections::HashMap;

/// An artist row: album artist when the tags have one, so compilations group together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistEntry {
    /// The first spelling met ("Aphex Twin" and "aphex twin" are one artist).
    pub name: String,
    pub tracks: usize,
    pub albums: usize,
}

/// An album tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumEntry {
    pub title: String,
    pub artist: String,
    pub year: Option<u16>,
    /// Indices into [`LibraryIndex::tracks`], in disc and track order.
    pub tracks: Vec<usize>,
}

/// Columns the track list sorts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrackSort {
    #[default]
    Artist,
    Title,
    Album,
    Duration,
    Added,
}

/// What the track list shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryFilter {
    /// Case-insensitive match on title, artist or album.
    pub text: String,
    /// Only this artist (as in [`ArtistEntry::name`]).
    pub artist: Option<String>,
    /// Only this album: (title, artist) as in [`AlbumEntry`].
    pub album: Option<(String, String)>,
}

struct Keys {
    title: String,
    artist: String,
    album: String,
}

/// The index over a snapshot of the library.
pub struct LibraryIndex {
    tracks: Vec<Track>,
    keys: Vec<Keys>,
    artists: Vec<ArtistEntry>,
    albums: Vec<AlbumEntry>,
}

fn artist_of(t: &Track) -> &str {
    t.metadata
        .album_artist
        .as_deref()
        .filter(|a| !a.trim().is_empty())
        .unwrap_or(&t.metadata.artist)
}

impl LibraryIndex {
    /// Indexes `tracks`.
    #[must_use]
    pub fn build(tracks: Vec<Track>) -> Self {
        let keys: Vec<Keys> = tracks
            .iter()
            .map(|t| Keys {
                title: t.metadata.title.to_lowercase(),
                artist: artist_of(t).to_lowercase(),
                album: t.metadata.album.as_deref().unwrap_or("").to_lowercase(),
            })
            .collect();

        let mut artist_at: HashMap<&str, usize> = HashMap::new();
        let mut artists: Vec<ArtistEntry> = Vec::new();
        let mut album_at: HashMap<(&str, &str), usize> = HashMap::new();
        let mut albums: Vec<AlbumEntry> = Vec::new();
        for (i, t) in tracks.iter().enumerate() {
            let k = &keys[i];
            let a = *artist_at.entry(k.artist.as_str()).or_insert_with(|| {
                artists.push(ArtistEntry {
                    name: artist_of(t).to_string(),
                    tracks: 0,
                    albums: 0,
                });
                artists.len() - 1
            });
            artists[a].tracks += 1;
            if k.album.trim().is_empty() {
                continue;
            }
            let b = *album_at
                .entry((k.artist.as_str(), k.album.as_str()))
                .or_insert_with(|| {
                    artists[a].albums += 1;
                    albums.push(AlbumEntry {
                        title: t.metadata.album.clone().unwrap_or_default(),
                        artist: artists[a].name.clone(),
                        year: t.metadata.year,
                        tracks: Vec::new(),
                    });
                    albums.len() - 1
                });
            albums[b].tracks.push(i);
            albums[b].year = albums[b].year.or(t.metadata.year);
        }
        for album in &mut albums {
            album.tracks.sort_by_key(|&i| {
                let m = &tracks[i].metadata;
                (
                    m.disc_number.unwrap_or(1),
                    m.track_number.unwrap_or(u32::MAX),
                )
            });
        }
        artists.sort_by_cached_key(|a| a.name.to_lowercase());
        albums.sort_by_cached_key(|a| (a.artist.to_lowercase(), a.year, a.title.to_lowercase()));
        Self {
            tracks,
            keys,
            artists,
            albums,
        }
    }

    /// Every track, in the order given to [`Self::build`].
    #[must_use]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Artists by name.
    #[must_use]
    pub fn artists(&self) -> &[ArtistEntry] {
        &self.artists
    }

    /// Albums by artist, year and title.
    #[must_use]
    pub fn albums(&self) -> &[AlbumEntry] {
        &self.albums
    }

    fn matches(&self, i: usize, filter: &LibraryFilter) -> bool {
        let k = &self.keys[i];
        if let Some(artist) = &filter.artist {
            if k.artist != artist.to_lowercase() {
                return false;
            }
        }
        if let Some((title, artist)) = &filter.album {
            if k.album != title.to_lowercase() || k.artist != artist.to_lowercase() {
                return false;
            }
        }
        let text = filter.text.trim().to_lowercase();
        text.is_empty()
            || k.title.contains(&text)
            || k.album.contains(&text)
            || k.artist.contains(&text)
            || self.tracks[i]
                .metadata
                .artist
                .to_lowercase()
                .contains(&text)
    }

    /// Indices of the tracks `filter` lets through, sorted by `sort`.
    #[must_use]
    pub fn tracks_sorted(
        &self,
        sort: TrackSort,
        ascending: bool,
        filter: &LibraryFilter,
    ) -> Vec<usize> {
        let mut rows: Vec<usize> = (0..self.tracks.len())
            .filter(|&i| self.matches(i, filter))
            .collect();
        let by = |a: &usize, b: &usize| {
            let (ka, kb) = (&self.keys[*a], &self.keys[*b]);
            let (ma, mb) = (&self.tracks[*a].metadata, &self.tracks[*b].metadata);
            match sort {
                TrackSort::Title => ka.title.cmp(&kb.title),
                TrackSort::Artist => ka
                    .artist
                    .cmp(&kb.artist)
                    .then_with(|| ka.album.cmp(&kb.album))
                    .then_with(|| ma.disc_number.cmp(&mb.disc_number))
                    .then_with(|| ma.track_number.cmp(&mb.track_number)),
                TrackSort::Album => ka
                    .album
                    .cmp(&kb.album)
                    .then_with(|| ma.disc_number.cmp(&mb.disc_number))
                    .then_with(|| ma.track_number.cmp(&mb.track_number)),
                TrackSort::Duration => ma.duration.cmp(&mb.duration),
                TrackSort::Added => self.tracks[*a]
                    .date_added_epoch
                    .cmp(&self.tracks[*b].date_added_epoch),
            }
        };
        if ascending {
            rows.sort_by(by);
        } else {
            rows.sort_by(|a, b| by(b, a));
        }
        rows
    }

    /// The `n` tracks added last, newest first.
    #[must_use]
    pub fn recently_added(&self, n: usize) -> Vec<usize> {
        let mut rows = self.tracks_sorted(TrackSort::Added, false, &LibraryFilter::default());
        rows.truncate(n);
        rows
    }

    /// How many tracks come from each source, most first.
    #[must_use]
    pub fn sources(&self) -> Vec<(SourceKind, usize)> {
        let mut counts: Vec<(SourceKind, usize)> = Vec::new();
        for t in &self.tracks {
            let kind = t.source.kind();
            match counts.iter_mut().find(|(k, _)| *k == kind) {
                Some((_, n)) => *n += 1,
                None => counts.push((kind, 1)),
            }
        }
        counts.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)))
        });
        counts
    }
}
