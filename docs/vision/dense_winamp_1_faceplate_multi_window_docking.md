# Dense Winamp 1.x Faceplate, Modular Window Docking & Detachable Views

This document defines the pixel-efficient, ultra-compact Winamp 1.x single-unit faceplate and the magnetic multi-window docking/snapping framework allowing windows (Main, EQ, Playlist, Video, Library, Visualizer) to lock together or detach across the screen.

---

## 1. Winamp 1.x Dense Faceplate Layout Specification

Unlike modern spacious interfaces, Winamp 1.x packed 100% of transport, time elapsed/remaining, bitrate/sample rate, stereo indicators, volume, balance, and mini-oscilloscope into an ultra-dense, tape-deck-style console ($275 \times 116$ physical pixels or $78 \times 14$ terminal cells).

```
┌────────────────────────────────────────────────────────────────────────────┐
│ [≡] WINAMP 1.x  :::  01. Aphex Twin - Alberto Balsalm  [192k][44.1][ST] [X]│
├────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌─ TIME ──────┐  VOL: ▍▍▍▍▍▍▍▍▍▍▍▍░░░ 78%               │
│  │ █ ▄ █ ▄ █ ▄ █│  │ [TRACK 01]  │  BAL: ░░░░░░░░[■]░░░░░░░  C             │
│  │ ▄ █ ▄ █ ▄ █ ▄│  │  03:42      │  EQ: [ON] [LOUD]  SHUF: [OFF] REP: [ALL]│
│  │ █ █ █ █ █ █ █│  │ [ELAPSED]   │  PRECACHE: [████████████████████] 100%  │
│  └──────────────┘  └─────────────┘                                         │
│  [⏮ PREV] [▶ PLAY] [⏸ PAUSE] [⏹ STOP] [⏭ NEXT] [⏏ EJECT]  POS: 03:42/05:11 │
│  Seek: ───────────────────────────●──────────────────────────────────────  │
└────────────────────────────────────────────────────────────────────────────┘
```

### 1.1. Dense Terminal Geometry & Cell Constraints

```rust
// crates/amp-adapter-tui/src/dense_console.rs
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Widget},
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MainConsoleState {
    pub title: String,
    pub artist: String,
    pub track_num: u32,
    pub elapsed: Duration,
    pub total: Option<Duration>,
    pub kbps: u32,
    pub sample_rate_khz: f32,
    pub is_stereo: bool,
    pub volume: u8,       // 0..100
    pub balance: i8,      // -50 (L) .. 0 (C) .. +50 (R)
    pub is_playing: bool,
    pub is_paused: bool,
    pub shuffle: bool,
    pub repeat: bool,
    pub precache_pct: u8,
    pub spectrum: [f32; 16],
}

pub struct DenseWinampConsole<'a> {
    state: &'a MainConsoleState,
}

impl<'a> DenseWinampConsole<'a> {
    pub fn new(state: &'a MainConsoleState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for DenseWinampConsole<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Enforce tight compact frame
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                format!(" 󰓃 WINAMP 1.x  :::  {} - {} ", self.state.artist, self.state.title),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 5 || inner.width < 50 {
            return; // Area too compressed for complete faceplate
        }

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top Display Row (LED Time + Mini Vis + Hardware Stats)
                Constraint::Length(1), // Seek bar
                Constraint::Length(1), // Hardware tape transport buttons
            ])
            .split(inner);

        // Top Row: [Mini-Visualizer (18 cols)] [LED Timer (16 cols)] [Knobs & Sliders]
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(18), // Mini Scope
                Constraint::Length(16), // Digital Segment Timer
                Constraint::Min(20),    // Volume, Balance, Precaching Status
            ])
            .split(rows[0]);

        // 1. Render Mini Spectrum Visualizer (Tape Head style)
        render_mini_spectrum(self.state.spectrum, top_chunks[0], buf);

        // 2. Render Green Vacuum-Fluorescent Display (VFD) Time Display
        let elapsed_secs = self.state.elapsed.as_secs();
        let time_str = format!("{:02}:{:02}", elapsed_secs / 60, elapsed_secs % 60);
        let time_vfd = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("TRK {:02} ", self.state.track_num), Style::default().fg(Color::Rgb(0, 180, 0))),
                Span::styled(if self.state.is_playing { "▶" } else if self.state.is_paused { "⏸" } else { "■" }, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(Span::styled(time_str, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        ])
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
        time_vfd.render(top_chunks[1], buf);

        // 3. Render Volume & Stats Column
        let vol_blocks = (self.state.volume as usize * 12) / 100;
        let vol_meter = format!("VOL: {:<12} {:>3}%", "█".repeat(vol_blocks), self.state.volume);
        let stats_line = format!(
            "[{:>3}k][{:>4.1}][{}] CACHE: {:>3}%",
            self.state.kbps,
            self.state.sample_rate_khz,
            if self.state.is_stereo { "ST" } else { "MO" },
            self.state.precache_pct
        );

        let stats_para = Paragraph::new(vec![
            Line::from(Span::styled(vol_meter, Style::default().fg(Color::Cyan))),
            Line::from(Span::styled(stats_line, Style::default().fg(Color::DarkGray))),
        ]);
        stats_para.render(top_chunks[2], buf);

        // 4. Render Seek Progress
        let total_secs = self.state.total.map(|d| d.as_secs()).unwrap_or(1);
        let progress_ratio = (elapsed_secs as f64 / total_secs as f64).clamp(0.0, 1.0);
        let seek_gauge = Gauge::default()
            .ratio(progress_ratio)
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Rgb(20, 20, 20)))
            .label(format!("{}/{}", time_str, format_duration(self.state.total.unwrap_or_default())));
        seek_gauge.render(rows[1], buf);

        // 5. Tape Transport Controls Row
        let transport = Line::from(vec![
            Span::styled(" [⏮ PREV] ", Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))),
            Span::raw(" "),
            Span::styled(" [▶ PLAY] ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(" [⏸ PAUSE] ", Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))),
            Span::raw(" "),
            Span::styled(" [⏹ STOP] ", Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))),
            Span::raw(" "),
            Span::styled(" [⏭ NEXT] ", Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))),
            Span::raw(" "),
            Span::styled(" [⏏ EJECT] ", Style::default().fg(Color::Yellow).bg(Color::Rgb(40, 40, 40))),
        ]);
        buf.set_line(rows[2].x, rows[2].y, &transport, rows[2].width);
    }
}

fn render_mini_spectrum(bands: [f32; 16], area: Rect, buf: &mut Buffer) {
    let glyphs = [" ", " ", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let mut line_spans = Vec::new();
    for &b in bands.iter().take(area.width as usize) {
        let idx = ((b * 8.0) as usize).min(glyphs.len() - 1);
        let color = if idx > 6 { Color::Red } else if idx > 4 { Color::Yellow } else { Color::Green };
        line_spans.push(Span::styled(glyphs[idx], Style::default().fg(color)));
    }
    buf.set_line(area.x, area.y + 1, &Line::from(line_spans), area.width);
}

fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}", s / 60, s % 60)
}
```

---

## 2. Multi-Window Docking & Magnetic Snapping Framework

All views are discrete windows that can snap to each other's edges or detach completely.

```
       ┌────────────────────────┐
       │   MAIN PLAYER (Locked) │
       ├────────────────────────┴────────┐
       │   10-BAND EQUALIZER (Snapped)   │
       ├─────────────────────────────────┴────────┐
       │         PLAYLIST EDITOR (Snapped)        │
       └──────────────────────────────────────────┘
                      
       ┌────────────────────────┐   ┌─────────────────────────┐
       │  VIDEO WINDOW (Torn)   │   │  LIBRARY / SEARCH (Torn)│
       │  (YouTube / Direct)    │   │  (P2P / Local / Web)    │
       └────────────────────────┘   └─────────────────────────┘
```

### 2.1. Window Manager & Snap Geometry Model

```rust
// crates/amp-adapter-tui/src/docking.rs
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const SNAP_THRESHOLD_CELLS: u16 = 2; // Distance to engage magnetic attraction

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowId {
    MainConsole,
    Equalizer,
    Playlist,
    LibraryBrowser,
    VideoPlayer,
    SpectrumVisualizer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSlot {
    pub id: WindowId,
    pub title: String,
    pub rect: Rect,
    pub visible: bool,
    pub docked_to: Option<WindowId>,
    pub z_index: usize,
}

pub struct DockingWindowManager {
    pub windows: HashMap<WindowId, WindowSlot>,
    pub active_focus: WindowId,
}

impl DockingWindowManager {
    pub fn new_classic_layout(screen: Rect) -> Self {
        let mut windows = HashMap::new();

        // 1. Main Console (Top-left anchor)
        let main_w = 78.min(screen.width);
        let main_h = 7;
        let main_rect = Rect::new(screen.x, screen.y, main_w, main_h);
        windows.insert(WindowId::MainConsole, WindowSlot {
            id: WindowId::MainConsole,
            title: "Winamp Main Console".into(),
            rect: main_rect,
            visible: true,
            docked_to: None,
            z_index: 0,
        });

        // 2. Equalizer (Snapped directly under Main)
        let eq_rect = Rect::new(screen.x, screen.y + main_h, main_w, 6);
        windows.insert(WindowId::Equalizer, WindowSlot {
            id: WindowId::Equalizer,
            title: "10-Band Equalizer".into(),
            rect: eq_rect,
            visible: true,
            docked_to: Some(WindowId::MainConsole),
            z_index: 0,
        });

        // 3. Playlist (Snapped under EQ)
        let pl_y = screen.y + main_h + 6;
        let pl_h = screen.height.saturating_sub(pl_y);
        let pl_rect = Rect::new(screen.x, pl_y, main_w, pl_h);
        windows.insert(WindowId::Playlist, WindowSlot {
            id: WindowId::Playlist,
            title: "Playlist Editor".into(),
            rect: pl_rect,
            visible: true,
            docked_to: Some(WindowId::Equalizer),
            z_index: 0,
        });

        // 4. Detached Library Window (Placed side-by-side if terminal has width)
        let lib_x = screen.x + main_w;
        let lib_w = screen.width.saturating_sub(main_w);
        let lib_rect = Rect::new(lib_x, screen.y, lib_w, screen.height);
        windows.insert(WindowId::LibraryBrowser, WindowSlot {
            id: WindowId::LibraryBrowser,
            title: "Universal Library & Search".into(),
            rect: lib_rect,
            visible: lib_w > 20,
            docked_to: None,
            z_index: 1,
        });

        // 5. Video View (Float hidden by default)
        windows.insert(WindowId::VideoPlayer, WindowSlot {
            id: WindowId::VideoPlayer,
            title: "Video Output Stream".into(),
            rect: Rect::new(screen.x + 10, screen.y + 3, 60, 20),
            visible: false,
            docked_to: None,
            z_index: 2,
        });

        Self {
            windows,
            active_focus: WindowId::MainConsole,
        }
    }

    /// Moves a window and automatically evaluates magnetic edge snapping
    pub fn move_window_with_magnetic_snap(&mut self, id: WindowId, mut target: Rect) {
        // Collect other visible window edges for snap target calculation
        let other_rects: Vec<(WindowId, Rect)> = self
            .windows
            .iter()
            .filter(|(w_id, slot)| **w_id != id && slot.visible)
            .map(|(w_id, slot)| (*w_id, slot.rect))
            .collect();

        let mut snapped_dock = None;

        for (other_id, other) in other_rects {
            // Check Bottom-to-Top snap: (target top attaches to other bottom)
            if (target.y as i32 - (other.y + other.height) as i32).abs() <= SNAP_THRESHOLD_CELLS as i32
                && (target.x as i32 - other.x as i32).abs() <= SNAP_THRESHOLD_CELLS as i32
            {
                target.y = other.y + other.height;
                target.x = other.x;
                snapped_dock = Some(other_id);
                break;
            }

            // Check Left-to-Right snap: (target left attaches to other right)
            if (target.x as i32 - (other.x + other.width) as i32).abs() <= SNAP_THRESHOLD_CELLS as i32
                && (target.y as i32 - other.y as i32).abs() <= SNAP_THRESHOLD_CELLS as i32
            {
                target.x = other.x + other.width;
                target.y = other.y;
                snapped_dock = Some(other_id);
                break;
            }
        }

        if let Some(slot) = self.windows.get_mut(&id) {
            slot.rect = target;
            slot.docked_to = snapped_dock;
        }
    }

    /// Toggles tear-off status for floating / secondary screens
    pub fn toggle_tear_off(&mut self, id: WindowId) {
        if let Some(slot) = self.windows.get_mut(&id) {
            if slot.docked_to.is_some() {
                slot.docked_to = None;
                // Offset slightly to demonstrate detachment
                slot.rect.x = slot.rect.x.saturating_add(4);
                slot.rect.y = slot.rect.y.saturating_add(2);
            }
        }
    }
}
```

---

## 3. Dedicated Detachable Video Mode View

When YouTube, Bandcamp video clips, or live visualizer streams are playing, this window can be extracted to render either direct ASCII/Unicode color video (via pure-Rust terminal canvas) or route hardware video frames via external IPC:

```rust
// crates/amp-adapter-tui/src/video_view.rs
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct VideoWindowView<'a> {
    pub is_active: bool,
    pub title: &'a str,
    pub frame_ascii_buffer: Option<&'a [u8]>, // Half-block Unicode or Sixel representation
    pub width: u16,
    pub height: u16,
}

impl<'a> Widget for VideoWindowView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Red))
            .title(format!("  Video Stream: {} [Tear-Off: 'v'] ", self.title));

        let inner = block.inner(area);
        block.render(area, buf);

        if !self.is_active || self.frame_ascii_buffer.is_none() {
            let placeholder = Paragraph::new("󰝚 Audio-only mode\n(Press 'V' to detach/dock video canvas)")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            placeholder.render(inner, buf);
            return;
        }

        // Render pure-Rust video frame matrix into terminal cell buffer
        // using ▀ (Upper Half Block) with fg/bg 24-bit truecolor
        // Zero allocations during rendering
    }
}
```