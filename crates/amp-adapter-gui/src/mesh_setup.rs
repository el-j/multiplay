//! The private mesh in the desktop app (TASK-167): join or leave, the mesh code (kept in the
//! encrypted vault), and the folders this device shares. The node runs on its own small tokio
//! runtime so it outlives any one search or download.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use amp_adapter_p2p::vault_keys::{IDENTITY, MESH_CODE};
use amp_adapter_p2p::{MeshCode, MeshConfig, MeshNode, MeshProvider, SharedFile};
use eframe::egui::{self, Color32, RichText, Ui};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Files shared at most; a bigger library shares its first ones.
const MAX_SHARED: usize = 20_000;
/// How often the member count is refreshed.
const PEER_REFRESH: Duration = Duration::from_secs(3);

/// The one mesh provider: search, the stream resolver and the setup share it.
pub fn mesh() -> Arc<MeshProvider> {
    static PROVIDER: OnceLock<Arc<MeshProvider>> = OnceLock::new();
    Arc::clone(PROVIDER.get_or_init(|| Arc::new(MeshProvider::default())))
}

fn runtime() -> Option<&'static tokio::runtime::Runtime> {
    static RT: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("multiplay-mesh")
            .build()
            .ok()
    })
    .as_ref()
}

enum MeshResult {
    Joined { node: MeshNode, shared: usize },
    Failed(String),
    Peers(usize),
}

/// Mesh state the P2P tab shows and changes.
#[derive(Default)]
pub struct MeshUi {
    /// A code typed to join someone else's mesh.
    pub code_input: String,
    /// Stay on 127.0.0.1 with a free port and no mDNS (tests).
    pub local_only: bool,
    pub(crate) status: String,
    pub(crate) joined: bool,
    /// The code of the joined mesh, to show and copy.
    pub(crate) code: Option<String>,
    started: bool,
    job: Option<Receiver<MeshResult>>,
    peers_at: Option<Instant>,
}

/// What the mesh controls ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshAction {
    /// Join (or leave) as the settings say.
    Apply,
    /// Join the mesh whose code was typed.
    Join,
    /// Start a new mesh with a fresh code.
    NewCode,
}

/// Audio files in `dirs` with their tags, for sharing.
#[must_use]
pub fn shared_files(dirs: &[String]) -> Vec<SharedFile> {
    dirs.iter()
        .flat_map(|d| amp_adapter_storage::scanner::collect_audio_files(std::path::Path::new(d)))
        .take(MAX_SHARED)
        .filter_map(|path: PathBuf| {
            let track = amp_adapter_storage::tags::extract_track_from_file(&path)?;
            Some(SharedFile {
                path,
                metadata: track.metadata,
            })
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

impl MultiplayGuiApp {
    /// One line on what the mesh is doing.
    #[must_use]
    pub fn mesh_status(&self) -> &str {
        &self.mesh_ui.status
    }

    /// The joined mesh's code.
    #[must_use]
    pub fn mesh_code(&self) -> Option<&str> {
        self.mesh_ui.code.as_deref()
    }

    /// True while joining.
    #[must_use]
    pub fn mesh_busy(&self) -> bool {
        self.mesh_ui.job.is_some()
    }

    /// True when a mesh search should run.
    #[must_use]
    pub fn mesh_joined(&self) -> bool {
        self.config.p2p.enable_p2p_mesh && self.mesh_ui.joined
    }

    fn start_mesh_job(
        &mut self,
        status: &str,
        job: impl std::future::Future<Output = MeshResult> + Send + 'static,
    ) {
        let Some(rt) = runtime() else {
            self.mesh_ui.status = "The mesh could not start on this computer".into();
            return;
        };
        let (tx, rx) = mpsc::channel();
        rt.spawn(async move {
            let _ = tx.send(job.await);
        });
        self.mesh_ui.job = Some(rx);
        if !status.is_empty() {
            self.mesh_ui.status = status.to_string();
        }
    }

    /// Makes the mesh match the settings: left, or joined with the saved code.
    pub fn apply_mesh(&mut self) {
        self.mesh_ui.started = true;
        self.mesh_ui.joined = false;
        self.mesh_ui.peers_at = None;
        if !self.config.p2p.enable_p2p_mesh {
            self.mesh_ui.status = "Off".into();
            self.mesh_ui.code = None;
            if let Some(rt) = runtime() {
                rt.spawn(async { mesh().leave().await });
            }
            return;
        }
        let code = match self
            .secret(MESH_CODE)
            .and_then(|c| c.parse::<MeshCode>().ok())
        {
            Some(c) => c,
            None => {
                let c = MeshCode::generate();
                self.save_secret(MESH_CODE, &c.to_string());
                c
            }
        };
        let identity = self
            .secret(IDENTITY)
            .and_then(|s| unhex(&s))
            .unwrap_or_else(|| {
                let id = amp_adapter_p2p::new_identity();
                self.save_secret(IDENTITY, &hex(&id));
                id
            });
        self.mesh_ui.code = Some(code.to_string());
        let dirs = self.config.p2p.shared_directories.clone();
        let config = MeshConfig {
            code,
            name: "multiplay".into(),
            listen_port: if self.mesh_ui.local_only {
                0
            } else {
                self.config.p2p.mesh_listen_port
            },
            discover_on_lan: !self.mesh_ui.local_only,
            shared: Vec::new(),
            identity: Some(identity),
        };
        self.start_mesh_job("Joining the mesh…", async move {
            let shared = tokio::task::spawn_blocking(move || shared_files(&dirs))
                .await
                .unwrap_or_default();
            let count = shared.len();
            match mesh().join(MeshConfig { shared, ..config }).await {
                Ok(node) => MeshResult::Joined {
                    node,
                    shared: count,
                },
                Err(e) => MeshResult::Failed(format!("The mesh could not start: {e}")),
            }
        });
    }

    /// Picks up a finished mesh step and refreshes the member count now and then.
    pub fn poll_mesh(&mut self) {
        if !self.mesh_ui.started && self.config.p2p.enable_p2p_mesh {
            self.apply_mesh();
        }
        let Some(job) = &self.mesh_ui.job else {
            if self.mesh_ui.joined
                && self
                    .mesh_ui
                    .peers_at
                    .is_none_or(|t| t.elapsed() >= PEER_REFRESH)
            {
                self.mesh_ui.peers_at = Some(Instant::now());
                self.start_mesh_job("", async {
                    match mesh().node().await {
                        Some(node) => MeshResult::Peers(node.peers().await.len()),
                        None => MeshResult::Failed("Not in a mesh".into()),
                    }
                });
            }
            return;
        };
        let result = match job.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => MeshResult::Failed("The mesh stopped".into()),
        };
        self.mesh_ui.job = None;
        match result {
            MeshResult::Joined { node, shared } => {
                self.mesh_ui.joined = true;
                self.mesh_ui.status = format!(
                    "In the mesh as {}, sharing {shared} files",
                    short(&node.peer_id())
                );
            }
            MeshResult::Peers(n) => {
                let base = self
                    .mesh_ui
                    .status
                    .split(" · ")
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let members = match n {
                    0 => "no one else here yet".to_string(),
                    1 => "1 other member".to_string(),
                    n => format!("{n} other members"),
                };
                self.mesh_ui.status = format!("{base} · {members}");
            }
            MeshResult::Failed(e) => {
                self.mesh_ui.joined = false;
                self.mesh_ui.status = e;
            }
        }
    }

    /// Runs a button from the mesh controls.
    pub fn mesh_action(&mut self, action: MeshAction) {
        match action {
            MeshAction::Apply => {}
            MeshAction::Join => match self.mesh_ui.code_input.trim().parse::<MeshCode>() {
                Ok(code) => {
                    self.save_secret(MESH_CODE, &code.to_string());
                    self.mesh_ui.code_input.clear();
                }
                Err(e) => {
                    self.mesh_ui.status = e.to_string();
                    return;
                }
            },
            MeshAction::NewCode => {
                self.save_secret(MESH_CODE, &MeshCode::generate().to_string());
            }
        }
        self.save_and_apply_config();
        self.apply_mesh();
    }
}

fn short(peer: &str) -> &str {
    &peer[peer.len().saturating_sub(6)..]
}

/// The mesh section of the P2P tab.
pub fn show_settings(
    ui: &mut Ui,
    config: &mut amp_domain::AppConfig,
    state: &mut MeshUi,
    action: &mut Option<MeshAction>,
) {
    ui.label(
        RichText::new("Private mesh")
            .strong()
            .color(Color32::from_rgb(0, 230, 140)),
    );
    let was = config.p2p.enable_p2p_mesh;
    ui.checkbox(
        &mut config.p2p.enable_p2p_mesh,
        "Join a private mesh (search and play music shared by your own devices and friends)",
    );
    if config.p2p.enable_p2p_mesh != was {
        *action = Some(MeshAction::Apply);
    }
    if !config.p2p.enable_p2p_mesh {
        return;
    }
    ui.label(
        RichText::new(
            "Only devices with the mesh code can connect; everything between them is encrypted. \
             Devices on the same network find each other by themselves.",
        )
        .size(10.0),
    );
    if let Some(code) = &state.code {
        ui.horizontal_wrapped(|ui| {
            ui.label("Mesh code");
            ui.label(RichText::new(code).monospace());
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(code.clone());
            }
            if ui
                .button("New code")
                .on_hover_text("Start a new mesh; others need the new code")
                .clicked()
            {
                *action = Some(MeshAction::NewCode);
            }
        });
    }
    ui.horizontal(|ui| {
        ui.label("Join another mesh");
        ui.add(egui::TextEdit::singleline(&mut state.code_input).hint_text("paste a mesh code"));
        if ui
            .add_enabled(
                !state.code_input.trim().is_empty(),
                egui::Button::new("Join"),
            )
            .clicked()
        {
            *action = Some(MeshAction::Join);
        }
    });
    let sharing = !config.p2p.shared_directories.is_empty();
    let mut share = sharing;
    ui.checkbox(&mut share, "Share my music folders with the mesh");
    if share != sharing {
        config.p2p.shared_directories = if share {
            config.library.music_directories.clone()
        } else {
            Vec::new()
        };
        *action = Some(MeshAction::Apply);
    }
    if !state.status.is_empty() {
        ui.label(RichText::new(&state.status).size(10.0));
    }
}
