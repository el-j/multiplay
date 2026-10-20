//! Remote & Sharing window (TASK-214): phone requests to moderate, paired devices, and the
//! pairing QR code. A playlist-style satellite window: docks, shades and resizes like the
//! playlist. Shares appear here once sharing over Soulseek/P2P is real (TASK-219).

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::docking::{DockRect, WindowId};
use crate::remote_requests::{display_title, Moderation, Preview, RequestRow};
use amp_adapter_remote::{DeviceId, DeviceSummary, RequestId};
use eframe::egui::{self, Color32, Layout, Pos2, RichText, Sense, Ui};

const ACCENT: Color32 = Color32::from_rgb(0, 235, 136);
const MUTED: Color32 = Color32::from_rgb(140, 150, 165);
const WARN: Color32 = Color32::from_rgb(240, 180, 0);
const BAD: Color32 = Color32::from_rgb(235, 90, 90);
const DECK_A: Color32 = Color32::from_rgb(0, 200, 255);
const DECK_B: Color32 = Color32::from_rgb(255, 150, 40);

/// Something the user did in the window, applied after drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowAction {
    Moderate(RequestId, Moderation),
    Revoke(DeviceId),
    RevokeAll,
    Trust(DeviceId, bool),
    NewCode,
    /// Starts remote control for this computer only (network access stays a Settings choice).
    TurnOn,
}

/// Applies `action` to the app and the daemon.
pub fn apply(app: &mut MultiplayGuiApp, action: WindowAction) {
    match action {
        WindowAction::Moderate(id, m) => {
            app.moderate(id, m);
        }
        WindowAction::Revoke(id) => {
            app.remote.revoke_device(&id);
            app.remote_requests.rows.retain(|r| r.device != id);
        }
        WindowAction::RevokeAll => {
            app.remote.revoke_all_sessions();
            app.remote_requests.rows.clear();
        }
        WindowAction::Trust(id, trusted) => {
            app.remote.set_trusted(&id, trusted);
        }
        WindowAction::NewCode => app.remote.refresh_token(),
        WindowAction::TurnOn => {
            app.config.remote.enabled = true;
            app.remote.apply_config(&app.config.remote);
        }
    }
}

/// What dropping a dragged request at `global` (screen points) does: onto the playlist adds it
/// to the end; onto the main window adds it too, or in DJ mode assigns it to the deck under
/// the pointer (left half A, right half B). `None` when it lands elsewhere.
#[must_use]
pub fn drop_target(app: &MultiplayGuiApp, global: Pos2) -> Option<Moderation> {
    let dm = &app.dock_manager;
    let hit = |id| dm.is_visible(id) && contains(&dm.get_rect(id), global);
    if hit(WindowId::Playlist) {
        return Some(Moderation::AddToEnd);
    }
    if hit(WindowId::Main) {
        if !app.dj_active() {
            return Some(Moderation::AddToEnd);
        }
        let main = dm.get_rect(WindowId::Main);
        let deck = usize::from(global.x >= main.x + main.width / 2.0);
        return Some(Moderation::Deck(deck));
    }
    None
}

fn contains(r: &DockRect, p: Pos2) -> bool {
    p.x >= r.left() && p.x < r.right() && p.y >= r.top() && p.y < r.bottom()
}

/// Window content below the title strip.
pub fn render(app: &mut MultiplayGuiApp, ui: &mut Ui) {
    let mut actions = Vec::new();
    if app.remote.is_running {
        status_line(app, ui);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                requests(app, ui, &mut actions);
                ui.add_space(6.0);
                devices(app, ui, &mut actions);
                ui.add_space(6.0);
                pairing(app, ui, &mut actions);
            });
    } else {
        switched_off(app, ui, &mut actions);
    }
    for action in actions {
        apply(app, action);
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).strong().color(ACCENT));
}

fn small(text: impl Into<String>, color: Color32) -> RichText {
    RichText::new(text.into()).size(10.0).color(color)
}

fn switched_off(app: &mut MultiplayGuiApp, ui: &mut Ui, actions: &mut Vec<WindowAction>) {
    heading(ui, "REMOTE CONTROL IS OFF");
    ui.label(small("Nothing is listening. Phones cannot connect.", MUTED));
    if let Some(err) = &app.remote.last_error {
        ui.label(small(err.clone(), BAD));
    }
    ui.add_space(4.0);
    if ui
        .button("Turn on for this computer")
        .on_hover_text("Phones on your Wi-Fi need network access: Settings → Remote")
        .clicked()
    {
        actions.push(WindowAction::TurnOn);
    }
    if ui.button("Settings → Remote…").clicked() {
        app.settings_tab = crate::settings_panel::GuiSettingsTab::Remote;
        app.dock_manager.set_visible(WindowId::Settings, true);
    }
}

fn status_line(app: &MultiplayGuiApp, ui: &mut Ui) {
    let scope = if app.remote.allow_lan {
        "your network"
    } else {
        "this computer only"
    };
    let text = format!(
        "● Listening on {scope} · {} connected",
        app.remote.active_peers_count()
    );
    ui.label(small(text, WARN));
    let mut uploads = app.remote.uploads_enabled();
    if ui
        .checkbox(&mut uploads, small("Let phones upload audio files", MUTED))
        .on_hover_text("Until remote control stops. Files up to 50 MB are checked, kept in a temporary folder and deleted afterwards.")
        .changed()
    {
        app.remote.set_uploads_enabled(uploads);
    }
}

fn requests(app: &mut MultiplayGuiApp, ui: &mut Ui, actions: &mut Vec<WindowAction>) {
    heading(
        ui,
        &format!("REQUESTS ({})", app.remote_requests.rows.len()),
    );
    if app.remote_requests.rows.is_empty() {
        ui.label(small(
            "No requests. Paired phones can search and ask for tracks.",
            MUTED,
        ));
        return;
    }
    let dj = app.dj_active();
    let rows = app.remote_requests.rows.clone();
    for (i, row) in rows.iter().enumerate() {
        let bg = if i % 2 == 0 {
            Color32::from_rgb(16, 22, 28)
        } else {
            Color32::from_rgb(12, 16, 22)
        };
        egui::Frame::NONE.fill(bg).show(ui, |ui| {
            ui.set_width(ui.available_width());
            request_row(app, ui, row, dj, actions);
        });
    }
}

fn request_row(
    app: &MultiplayGuiApp,
    ui: &mut Ui,
    row: &RequestRow,
    dj: bool,
    actions: &mut Vec<WindowAction>,
) {
    let ready = matches!(row.preview, Preview::Ready(_));
    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
        let reject = RichText::new("✕").size(10.0).color(BAD);
        if ui.small_button(reject).on_hover_text("Reject").clicked() {
            actions.push(WindowAction::Moderate(row.id, Moderation::Reject));
        }
        let mut accept = |ui: &mut Ui, text: RichText, tip: &str, m: Moderation| {
            let b = ui.add_enabled(ready, egui::Button::new(text).small());
            if b.on_hover_text(tip).clicked() {
                actions.push(WindowAction::Moderate(row.id, m));
            }
        };
        if dj {
            accept(
                ui,
                small("B", DECK_B),
                "Assign to deck B",
                Moderation::Deck(1),
            );
            accept(
                ui,
                small("A", DECK_A),
                "Assign to deck A",
                Moderation::Deck(0),
            );
        }
        accept(ui, small("⏭", ACCENT), "Play next", Moderation::PlayNext);
        accept(
            ui,
            small("+", ACCENT),
            "Add to the end of the playlist",
            Moderation::AddToEnd,
        );
        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(small(&row.device_name, WARN));
            let (text, color) = match &row.preview {
                Preview::Ready(t) => (
                    format!("{}{}", display_title(t), duration(t)),
                    Color32::WHITE,
                ),
                Preview::Resolving => (format!("Looking up {}…", row.asked_for()), MUTED),
                Preview::Failed(why) => (format!("{}: {why}", row.asked_for()), BAD),
            };
            let label = egui::Label::new(RichText::new(&text).size(11.0).color(color))
                .sense(Sense::click_and_drag());
            let response = ui.add(label).on_hover_text(if ready {
                "Drag onto the playlist or a deck"
            } else {
                text.as_str()
            });
            if ready {
                drag_to_drop(app, ui, &response, row, &text, actions);
            }
        });
    });
}

fn duration(t: &amp_domain::Track) -> String {
    t.metadata.duration.map_or(String::new(), |d| {
        format!("  {}:{:02}", d.as_secs() / 60, d.as_secs() % 60)
    })
}

/// Drag a ready request out of the window: a ghost follows the pointer, and on release the
/// drop position (in screen points) picks the playlist or a deck.
fn drag_to_drop(
    app: &MultiplayGuiApp,
    ui: &Ui,
    response: &egui::Response,
    row: &RequestRow,
    text: &str,
    actions: &mut Vec<WindowAction>,
) {
    let ctx = ui.ctx();
    let memory = egui::Id::unique(("remote_drag_pos", row.id.0));
    if response.dragged() {
        if let Some(pos) = ctx.pointer_latest_pos() {
            ctx.data_mut(|d| d.insert_temp(memory, pos));
            let layer = egui::LayerId::new(egui::Order::Tooltip, memory);
            ctx.layer_painter(layer).text(
                pos + egui::vec2(12.0, 0.0),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(11.0),
                ACCENT,
            );
        }
    }
    if response.drag_stopped() {
        let last = ctx.data(|d| d.get_temp::<Pos2>(memory));
        let pos = ctx.pointer_latest_pos().or(last);
        ctx.data_mut(|d| d.remove::<Pos2>(memory));
        let origin = app.dock_manager.get_rect(WindowId::Remote);
        if let Some(pos) = pos {
            let global = Pos2::new(origin.x + pos.x, origin.y + pos.y);
            if let Some(m) = drop_target(app, global) {
                actions.push(WindowAction::Moderate(row.id, m));
            }
        }
    }
}

fn devices(app: &MultiplayGuiApp, ui: &mut Ui, actions: &mut Vec<WindowAction>) {
    let list = app.remote.devices();
    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
        if !list.is_empty()
            && ui
                .small_button(small("Revoke all", BAD))
                .on_hover_text("Every phone must scan a new code")
                .clicked()
        {
            actions.push(WindowAction::RevokeAll);
        }
        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            heading(ui, &format!("DEVICES ({})", list.len()));
        });
    });
    if list.is_empty() {
        ui.label(small(
            "No paired phones yet. Scan the code below to pair one.",
            MUTED,
        ));
    }
    let now = amp_adapter_remote::devices::unix_now();
    for device in &list {
        device_row(ui, device, now, actions);
    }
}

fn device_row(ui: &mut Ui, d: &DeviceSummary, now: u64, actions: &mut Vec<WindowAction>) {
    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
        if ui
            .small_button(small("Revoke", BAD))
            .on_hover_text("Forget this phone and disconnect it")
            .clicked()
        {
            actions.push(WindowAction::Revoke(d.id.clone()));
        }
        let mut trusted = d.trusted;
        if ui
            .checkbox(&mut trusted, small("auto-accept", MUTED))
            .on_hover_text("Add this phone's requests to the playlist without asking")
            .changed()
        {
            actions.push(WindowAction::Trust(d.id.clone(), trusted));
        }
        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            let (dot, seen) = if d.connected {
                (ACCENT, "connected".to_string())
            } else {
                (MUTED, format!("seen {}", ago(now, d.last_seen)))
            };
            ui.label(RichText::new("●").size(10.0).color(dot));
            ui.label(RichText::new(&d.name).size(11.0).strong());
            ui.label(small(seen, MUTED));
        });
    });
}

/// "just now", "5 min ago", "3 h ago", "2 d ago"; "earlier" when unknown.
fn ago(now: u64, then: u64) -> String {
    if then == 0 {
        return "earlier".into();
    }
    match now.saturating_sub(then) {
        s if s < 60 => "just now".into(),
        s if s < 3_600 => format!("{} min ago", s / 60),
        s if s < 86_400 => format!("{} h ago", s / 3_600),
        s => format!("{} d ago", s / 86_400),
    }
}

fn pairing(app: &MultiplayGuiApp, ui: &mut Ui, actions: &mut Vec<WindowAction>) {
    heading(ui, "PAIR A PHONE");
    if !app.remote.allow_lan {
        ui.label(small(
            "Only this computer can connect. Allow your network in Settings → Remote.",
            WARN,
        ));
    }
    ui.horizontal_wrapped(|ui| {
        if let Some(matrix) = &app.remote.qr_matrix {
            let size = (ui.available_width() - 4.0).clamp(60.0, 120.0);
            crate::qr_widget::draw_qr_code(ui, matrix, size);
        }
        ui.vertical(|ui| {
            ui.label(small("Scan with the phone camera.", MUTED));
            ui.label(small("Each code works once, for 10 minutes.", MUTED));
            ui.label(RichText::new(format!("Code: {}", app.remote.current_token)).monospace());
            if ui.small_button("New code").clicked() {
                actions.push(WindowAction::NewCode);
            }
            let mut url = app.remote.pairing_url();
            ui.add(egui::TextEdit::singleline(&mut url).desired_width(ui.available_width()));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_seen_is_readable() {
        assert_eq!(ago(1_000, 990), "just now");
        assert_eq!(ago(10_000, 10_000 - 300), "5 min ago");
        assert_eq!(ago(100_000, 100_000 - 7_200), "2 h ago");
        assert_eq!(ago(1_000_000, 1_000_000 - 172_800), "2 d ago");
        assert_eq!(ago(5, 0), "earlier");
    }
}
