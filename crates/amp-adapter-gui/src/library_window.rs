//! The Library window (TASK-172): Artists, Albums (cover grid), Tracks (sortable), Recently
//! Added and Sources. Lists draw only their visible rows, so 100k tracks scroll smoothly; covers
//! are made on a worker thread and kept in a fixed number of textures.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use amp_adapter_storage::artwork::{ArtworkCache, THUMBNAIL_PX};
use amp_core_engine::library_index::{LibraryFilter, TrackSort};
use amp_domain::Track;
use eframe::egui::{self, Color32, RichText, TextureHandle, Ui, Vec2};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

/// Cover textures kept at once (160 px RGBA is 100 KB each, so about 12 MB).
const MAX_COVERS: usize = 120;
/// Disk budget of the thumbnail cache.
const ARTWORK_CACHE_BYTES: u64 = 64 << 20;
const ROW: f32 = 18.0;
const TILE: f32 = 120.0;
const RECENT: usize = 200;

/// A least-recently-used map with a fixed capacity.
pub struct CoverLru<T> {
    cap: usize,
    tick: u64,
    items: HashMap<String, (T, u64)>,
}

impl<T> CoverLru<T> {
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            tick: 0,
            items: HashMap::new(),
        }
    }

    /// Adds `value`, dropping the least recently used entry when full.
    pub fn insert(&mut self, key: String, value: T) {
        if !self.items.contains_key(&key) && self.items.len() >= self.cap {
            if let Some(oldest) = self
                .items
                .iter()
                .min_by_key(|(_, (_, t))| *t)
                .map(|(k, _)| k.clone())
            {
                self.items.remove(&oldest);
            }
        }
        self.tick += 1;
        self.items.insert(key, (value, self.tick));
    }

    /// Marks `key` as just used.
    pub fn touch(&mut self, key: &str) {
        self.tick += 1;
        if let Some(entry) = self.items.get_mut(key) {
            entry.1 = self.tick;
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&T> {
        self.items.get(key).map(|(v, _)| v)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Library window tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryTab {
    Artists,
    #[default]
    Albums,
    Tracks,
    Recent,
    Sources,
}

type CoverResult = (String, Option<egui::ColorImage>);
/// Requests to and results from the cover worker thread.
type CoverWorker = (Sender<(String, Track)>, Receiver<CoverResult>);
/// What the cached track rows were sorted for: index generation, filter, sort, ascending.
type RowsKey = (u64, LibraryFilter, TrackSort, bool);

/// View state of the Library window.
pub struct LibraryUi {
    pub tab: LibraryTab,
    pub filter: LibraryFilter,
    pub sort: TrackSort,
    pub ascending: bool,
    /// Sorted rows for (generation, filter, sort, ascending).
    rows: Option<(RowsKey, Vec<usize>)>,
    covers: CoverLru<TextureHandle>,
    requested: HashSet<String>,
    no_cover: HashSet<String>,
    worker: Option<CoverWorker>,
}

impl Default for LibraryUi {
    fn default() -> Self {
        Self {
            tab: LibraryTab::default(),
            filter: LibraryFilter::default(),
            sort: TrackSort::Artist,
            ascending: true,
            rows: None,
            covers: CoverLru::new(MAX_COVERS),
            requested: HashSet::new(),
            no_cover: HashSet::new(),
            worker: None,
        }
    }
}

fn start_cover_worker(cache: ArtworkCache, ctx: egui::Context) -> Option<CoverWorker> {
    let (req_tx, req_rx) = mpsc::channel::<(String, Track)>();
    let (res_tx, res_rx) = mpsc::channel::<CoverResult>();
    std::thread::Builder::new()
        .name("multiplay-covers".into())
        .spawn(move || {
            for (key, track) in req_rx {
                let image = cache.thumbnail(&track).and_then(|png| {
                    let img = image::open(png).ok()?.to_rgba8();
                    let size = [img.width() as usize, img.height() as usize];
                    Some(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
                });
                if res_tx.send((key, image)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        })
        .ok()?;
    Some((req_tx, res_rx))
}

impl MultiplayGuiApp {
    /// Rows of the track list for the current filter and sort (cached until either changes).
    fn library_rows(&mut self) -> Vec<usize> {
        let Some(index) = &self.library_index else {
            return Vec::new();
        };
        let ui = &mut self.library_ui;
        let key = (
            self.library_generation,
            ui.filter.clone(),
            ui.sort,
            ui.ascending,
        );
        if ui.rows.as_ref().is_none_or(|(k, _)| *k != key) {
            let rows = index.tracks_sorted(ui.sort, ui.ascending, &ui.filter);
            ui.rows = Some((key, rows));
        }
        ui.rows.as_ref().map(|(_, r)| r.clone()).unwrap_or_default()
    }

    /// The cover texture for an album tile, asking the worker for it the first time.
    fn album_cover(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        track: &Track,
    ) -> Option<TextureHandle> {
        let lib = &mut self.library_ui;
        if lib.worker.is_none() {
            let cache =
                ArtworkCache::new(&self.paths.data_dir.join("artwork"), ARTWORK_CACHE_BYTES);
            lib.worker = start_cover_worker(cache, ctx.clone());
        }
        if let Some((_, results)) = &lib.worker {
            for (k, image) in results.try_iter() {
                lib.requested.remove(&k);
                match image {
                    Some(img) => {
                        let tex = ctx.load_texture(
                            format!("cover-{k}"),
                            img,
                            egui::TextureOptions::LINEAR,
                        );
                        lib.covers.insert(k, tex);
                    }
                    None => {
                        lib.no_cover.insert(k);
                    }
                }
            }
        }
        if let Some(tex) = lib.covers.get(key).cloned() {
            lib.covers.touch(key);
            return Some(tex);
        }
        if !lib.no_cover.contains(key) && lib.requested.insert(key.to_string()) {
            if let Some((tx, _)) = &lib.worker {
                let _ = tx.send((key.to_string(), track.clone()));
            }
        }
        None
    }
}

fn mmss(d: Option<Duration>) -> String {
    d.map_or_else(String::new, |d| {
        format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60)
    })
}

/// Draws the Library window.
pub fn render_library_window(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    if app.library_index.is_none() {
        app.load_library();
        ui.label("Loading the library…");
        return;
    }
    ui.horizontal_wrapped(|ui| {
        let tab = &mut app.library_ui.tab;
        for (t, label) in [
            (LibraryTab::Artists, "Artists"),
            (LibraryTab::Albums, "Albums"),
            (LibraryTab::Tracks, "Tracks"),
            (LibraryTab::Recent, "Recently added"),
            (LibraryTab::Sources, "Sources"),
        ] {
            ui.selectable_value(tab, t, label);
        }
    });
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.library_ui.filter.text)
                .hint_text("Filter title, artist or album")
                .desired_width((ui.available_width() - 90.0).max(60.0)),
        );
        let narrowed =
            app.library_ui.filter.artist.is_some() || app.library_ui.filter.album.is_some();
        if narrowed && ui.button("Show all").clicked() {
            app.library_ui.filter.artist = None;
            app.library_ui.filter.album = None;
        }
    });
    let count = app.library_index.as_ref().map_or(0, |i| i.tracks().len());
    if count == 0 {
        ui.add_space(8.0);
        ui.label("The library is empty. Drop music onto any window, or use + in the playlist: everything you add is remembered here.");
        return;
    }
    ui.separator();
    match app.library_ui.tab {
        LibraryTab::Artists => artists_view(app, ui),
        LibraryTab::Albums => albums_view(app, ui),
        LibraryTab::Tracks => tracks_view(app, ui),
        LibraryTab::Recent => recent_view(app, ui),
        LibraryTab::Sources => sources_view(app, ui),
    }
}

fn artists_view(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let Some(index) = &app.library_index else {
        return;
    };
    let text = app.library_ui.filter.text.to_lowercase();
    let artists: Vec<_> = index
        .artists()
        .iter()
        .filter(|a| text.is_empty() || a.name.to_lowercase().contains(&text))
        .cloned()
        .collect();
    let mut picked = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW, artists.len(), |ui, range| {
            for a in &artists[range] {
                let line = format!("{}   ·   {} albums, {} tracks", a.name, a.albums, a.tracks);
                if ui
                    .selectable_label(false, line)
                    .on_hover_text("Show this artist's tracks")
                    .clicked()
                {
                    picked = Some(a.name.clone());
                }
            }
        });
    if let Some(name) = picked {
        app.library_ui.filter = LibraryFilter {
            artist: Some(name),
            ..Default::default()
        };
        app.library_ui.sort = TrackSort::Album;
        app.library_ui.tab = LibraryTab::Tracks;
    }
}

fn albums_view(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let Some(index) = &app.library_index else {
        return;
    };
    let text = app.library_ui.filter.text.to_lowercase();
    let albums: Vec<_> = index
        .albums()
        .iter()
        .filter(|a| {
            text.is_empty()
                || a.title.to_lowercase().contains(&text)
                || a.artist.to_lowercase().contains(&text)
        })
        .map(|a| (a.clone(), index.tracks()[a.tracks[0]].clone()))
        .collect();
    let per_row = ((ui.available_width() / (TILE + 8.0)).floor() as usize).max(1);
    let rows = albums.len().div_ceil(per_row);
    let ctx = ui.ctx().clone();
    let mut action = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, TILE + 40.0, rows, |ui, range| {
            for r in range {
                ui.horizontal(|ui| {
                    for (album, first) in albums.iter().skip(r * per_row).take(per_row) {
                        ui.vertical(|ui| {
                            ui.set_width(TILE);
                            let key = first.library_key();
                            let size = Vec2::splat(TILE.min(THUMBNAIL_PX as f32));
                            let resp = match app.album_cover(&ctx, &key, first) {
                                Some(tex) => ui.add(
                                    egui::Image::new(&tex)
                                        .fit_to_exact_size(size)
                                        .sense(egui::Sense::click()),
                                ),
                                None => ui.add(
                                    egui::Button::new(RichText::new("♪").size(40.0)).min_size(size),
                                ),
                            };
                            let resp = resp.on_hover_text(format!(
                                "{} — {}\nClick: show tracks · Double-click: play",
                                album.title, album.artist
                            ));
                            if resp.double_clicked() {
                                action = Some((album.clone(), true));
                            } else if resp.clicked() {
                                action = Some((album.clone(), false));
                            }
                            ui.add(
                                egui::Label::new(RichText::new(&album.title).size(11.0).strong())
                                    .truncate(),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&album.artist).size(10.0).color(Color32::GRAY),
                                )
                                .truncate(),
                            );
                        });
                    }
                });
            }
        });
    match action {
        Some((album, true)) => app.library_play_rows(&album.tracks),
        Some((album, false)) => {
            app.library_ui.filter = LibraryFilter {
                album: Some((album.title, album.artist)),
                ..Default::default()
            };
            app.library_ui.sort = TrackSort::Album;
            app.library_ui.ascending = true;
            app.library_ui.tab = LibraryTab::Tracks;
        }
        None => {}
    }
}

fn track_rows(app: &mut MultiplayGuiApp, ui: &mut Ui, rows: &[usize], sortable: bool) {
    if sortable {
        ui.horizontal(|ui| {
            for (sort, label) in [
                (TrackSort::Artist, "Artist"),
                (TrackSort::Title, "Title"),
                (TrackSort::Album, "Album"),
                (TrackSort::Duration, "Length"),
                (TrackSort::Added, "Added"),
            ] {
                let lib = &mut app.library_ui;
                let arrow = if lib.sort == sort {
                    if lib.ascending {
                        " ⬆"
                    } else {
                        " ⬇"
                    }
                } else {
                    ""
                };
                if ui
                    .selectable_label(lib.sort == sort, format!("{label}{arrow}"))
                    .clicked()
                {
                    if lib.sort == sort {
                        lib.ascending = !lib.ascending;
                    } else {
                        lib.sort = sort;
                        lib.ascending = true;
                    }
                }
            }
            if ui
                .button("+ Add all")
                .on_hover_text("Add these tracks to the playlist")
                .clicked()
            {
                app.library_add_rows(rows);
            }
        });
    }
    let Some(index) = &app.library_index else {
        return;
    };
    let mut play = None;
    let mut add = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW, rows.len(), |ui, range| {
            for &i in &rows[range] {
                let t = &index.tracks()[i];
                let m = &t.metadata;
                let line = format!(
                    "{} - {}   {}   {}",
                    m.artist,
                    m.title,
                    m.album.as_deref().unwrap_or(""),
                    mmss(m.duration)
                );
                let resp = ui
                    .add(
                        egui::Label::new(line)
                            .truncate()
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_text("Double-click: play · Right-click: add to the playlist");
                if resp.double_clicked() {
                    play = Some(i);
                } else if resp.secondary_clicked() {
                    add = Some(i);
                }
            }
        });
    if let Some(i) = play {
        app.library_play_rows(&[i]);
    }
    if let Some(i) = add {
        app.library_add_rows(&[i]);
    }
}

fn tracks_view(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let rows = app.library_rows();
    ui.label(RichText::new(format!("{} tracks", rows.len())).size(10.0));
    track_rows(app, ui, &rows, true);
}

fn recent_view(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let rows = app
        .library_index
        .as_ref()
        .map(|i| i.recently_added(RECENT))
        .unwrap_or_default();
    track_rows(app, ui, &rows, false);
}

fn sources_view(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let Some(index) = &app.library_index else {
        return;
    };
    for (kind, n) in index.sources() {
        ui.label(format!("{kind:?}: {n} tracks"));
    }
}
