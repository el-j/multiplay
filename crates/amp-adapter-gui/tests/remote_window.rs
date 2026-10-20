//! The Remote & Sharing window (TASK-214): remembered in the layout, fits at every size,
//! and its device actions reach the daemon (revoking one phone leaves the others connected).

#![forbid(unsafe_code)]

use amp_adapter_gui::chrome::layout::TITLE_HEIGHT;
use amp_adapter_gui::chrome::resize::min_inner_size;
use amp_adapter_gui::layout_manager::LayoutSnapshot;
use amp_adapter_gui::remote_requests::{Moderation, Preview, RequestRow};
use amp_adapter_gui::remote_window::{apply, drop_target, WindowAction};
use amp_adapter_gui::viewports::window_body;
use amp_adapter_gui::{DockRect, MultiplayGuiApp, WindowId};
use amp_adapter_remote::{DeviceId, RequestId, ValidReference};
use amp_domain::{AudioSource, RemoteControlConfig, Track, TrackMetadata};
use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};
use std::collections::HashMap;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::{self, Message, WebSocket};

struct Host {
    app: MultiplayGuiApp,
    dir: PathBuf,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn host(remote_on: bool) -> Host {
    let dir = std::env::temp_dir().join(format!("multiplay-remote-win-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = MultiplayGuiApp::headless(&dir).expect("headless app");
    if remote_on {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        app.config.remote = RemoteControlConfig {
            enabled: true,
            allow_lan: false,
            port,
        };
        app.remote.apply_config(&app.config.remote);
        assert!(app.remote.is_running, "{:?}", app.remote.last_error);
    }
    Host { app, dir }
}

fn phone(app: &mut MultiplayGuiApp, user_agent: &'static str) -> WebSocket<TcpStream> {
    let port = app.remote.port;
    let token = app.remote.current_token.clone();
    app.remote.refresh_token();
    let mut req = format!("ws://127.0.0.1:{port}/ws?token={token}")
        .into_client_request()
        .unwrap();
    let origin = HeaderValue::from_str(&format!("http://127.0.0.1:{port}")).unwrap();
    req.headers_mut().insert("Origin", origin);
    req.headers_mut()
        .insert("User-Agent", HeaderValue::from_static(user_agent));
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    tcp.set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let (ws, _) = tungstenite::client::client(req, tcp).expect("pairing");
    ws
}

/// True once the host closed the phone's socket (within 3 s).
fn closed(ws: &mut WebSocket<TcpStream>) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        match ws.read() {
            Ok(Message::Close(_)) => return true,
            Ok(_) | Err(tungstenite::Error::Io(_)) => {}
            Err(_) => return true,
        }
    }
    false
}

fn wait_for_devices(app: &MultiplayGuiApp, n: usize) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while app.remote.devices().iter().filter(|d| d.connected).count() < n {
        assert!(Instant::now() < deadline, "devices never connected");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// How far the Remote window's content spills past `size` (0 when it fits).
fn overflow(app: &mut MultiplayGuiApp, size: Vec2) -> Vec2 {
    let ctx = egui::Context::default();
    let mut spill = Vec2::ZERO;
    for _ in 0..3 {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::Panel::top("title")
                .exact_size(TITLE_HEIGHT)
                .show(ui, |_| {});
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(8.0))
                .show(ui, |ui| {
                    let avail = ui.max_rect();
                    window_body(app, WindowId::Remote, ui);
                    let used = ui.min_rect();
                    spill = Vec2::new(
                        (used.right() - avail.right()).max(0.0),
                        (used.bottom() - avail.bottom()).max(0.0),
                    );
                });
        });
        output.textures_delta.clear();
    }
    spill
}

fn long_row(id: u64, preview: Preview) -> RequestRow {
    RequestRow {
        id: RequestId(id),
        device: DeviceId("0011aabb".into()),
        device_name: "A Phone With A Remarkably Long Device Name".into(),
        reference: ValidReference::Search("an extremely long search query nobody types".into()),
        preview,
        auto_accept: false,
        generation: 1,
    }
}

#[test]
fn remote_window_is_remembered_in_the_layout() {
    let h = host(false);
    let mut dm = h.app.dock_manager.clone();
    assert!(!dm.is_visible(WindowId::Remote), "hidden by default");
    dm.set_visible(WindowId::Remote, true);
    dm.set_rect(WindowId::Remote, DockRect::new(700.0, 120.0, 340.0, 300.0));
    let snap = LayoutSnapshot::capture(&dm, &HashMap::new(), 1.0);
    let mut back = h.app.dock_manager.clone();
    snap.apply(&mut back, &mut HashMap::new());
    assert!(back.is_visible(WindowId::Remote));
    assert_eq!(
        back.get_rect(WindowId::Remote),
        dm.get_rect(WindowId::Remote)
    );
}

#[test]
fn remote_window_fits_from_its_minimum_up() {
    // Off, then on with long request rows and paired devices.
    let mut off = host(false);
    let min = min_inner_size(WindowId::Remote);
    let spill = overflow(&mut off.app, min);
    assert!(spill.x <= 0.5 && spill.y <= 0.5, "off: spills {spill:?}");

    let mut h = host(true);
    let _a = phone(&mut h.app, "Mozilla/5.0 (iPhone)");
    let _b = phone(&mut h.app, "Mozilla/5.0 (Linux; Android 15) Mobile");
    wait_for_devices(&h.app, 2);
    let long = "An Extremely Long Track Title That Would Never Fit (Extended Mix)";
    let generation = h.app.remote.generation;
    for (i, preview) in [
        Preview::Resolving,
        Preview::Failed("Nothing found for this very long query string".into()),
        Preview::Ready(Box::new(Track::new(
            AudioSource::YouTube {
                video_id: "dQw4w9WgXcQ".into(),
            },
            TrackMetadata::new(long, "A Very Long Artist Name Featuring Another Artist"),
        ))),
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = long_row(i as u64 + 1, preview);
        row.generation = generation;
        h.app.remote_requests.rows.push(row);
    }
    for dj in [false, true] {
        if dj {
            h.app.toggle_dj_mode(None);
        }
        for size in [min, Vec2::new(420.0, 360.0), Vec2::new(800.0, 700.0)] {
            let spill = overflow(&mut h.app, size);
            assert!(
                spill.x <= 0.5 && spill.y <= 0.5,
                "dj={dj}: Remote window overflows at {size:?} by {spill:?}"
            );
        }
    }
}

#[test]
fn remote_window_revoking_one_device_keeps_the_others() {
    let mut h = host(true);
    let mut iphone = phone(&mut h.app, "Mozilla/5.0 (iPhone)");
    let mut android = phone(&mut h.app, "Mozilla/5.0 (Linux; Android 15) Mobile");
    wait_for_devices(&h.app, 2);
    let target = h
        .app
        .remote
        .devices()
        .into_iter()
        .find(|d| d.name == "iPhone")
        .unwrap();

    apply(&mut h.app, WindowAction::Revoke(target.id.clone()));
    assert!(closed(&mut iphone), "the revoked phone is disconnected");
    let left = h.app.remote.devices();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].name, "Android phone");
    assert!(left[0].connected);
    assert!(!closed(&mut android), "the other phone stays connected");

    apply(&mut h.app, WindowAction::Trust(left[0].id.clone(), true));
    assert!(h.app.remote.devices()[0].trusted);
    apply(&mut h.app, WindowAction::RevokeAll);
    assert!(closed(&mut android));
    assert!(h.app.remote.devices().is_empty());
}

#[test]
fn remote_window_turn_on_and_new_code() {
    let mut h = host(false);
    h.app.config.remote.port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    apply(&mut h.app, WindowAction::TurnOn);
    assert!(h.app.remote.is_running);
    assert!(h.app.config.remote.enabled);
    assert!(
        !h.app.config.remote.allow_lan,
        "turning on from here stays on this computer"
    );
    let old = h.app.remote.current_token.clone();
    apply(&mut h.app, WindowAction::NewCode);
    assert_ne!(h.app.remote.current_token, old);
}

#[test]
fn remote_window_moderate_action_rejects() {
    let mut h = host(true);
    let mut row = long_row(1, Preview::Resolving);
    row.generation = h.app.remote.generation;
    h.app.remote_requests.rows.push(row);
    apply(
        &mut h.app,
        WindowAction::Moderate(RequestId(1), Moderation::Reject),
    );
    assert!(h.app.remote_requests.rows.is_empty());
    assert!(h.app.playlist.is_empty());
}

#[test]
fn remote_window_drop_targets() {
    let mut h = host(false);
    let dm = &mut h.app.dock_manager;
    dm.set_rect(WindowId::Main, DockRect::new(100.0, 100.0, 500.0, 230.0));
    dm.set_rect(
        WindowId::Playlist,
        DockRect::new(100.0, 330.0, 500.0, 300.0),
    );
    dm.set_visible(WindowId::Playlist, true);

    assert_eq!(
        drop_target(&h.app, Pos2::new(300.0, 400.0)),
        Some(Moderation::AddToEnd)
    );
    assert_eq!(drop_target(&h.app, Pos2::new(1500.0, 50.0)), None);
    assert_eq!(
        drop_target(&h.app, Pos2::new(150.0, 200.0)),
        Some(Moderation::AddToEnd),
        "outside DJ mode the main window queues it"
    );

    h.app.toggle_dj_mode(None);
    let main = h.app.dock_manager.get_rect(WindowId::Main);
    let y = main.y + main.height / 2.0;
    assert_eq!(
        drop_target(&h.app, Pos2::new(main.x + 20.0, y)),
        Some(Moderation::Deck(0))
    );
    assert_eq!(
        drop_target(&h.app, Pos2::new(main.right() - 20.0, y)),
        Some(Moderation::Deck(1))
    );
    h.app.dock_manager.set_visible(WindowId::Playlist, false);
    assert_eq!(
        drop_target(&h.app, Pos2::new(300.0, 400.0)),
        None,
        "hidden windows take nothing"
    );
}
