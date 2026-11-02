//! The private mesh in the app (TASK-167): off by default; on, it joins with a code kept in the
//! vault, shares only the chosen folders, and is searched once joined.

#![forbid(unsafe_code)]

use amp_adapter_gui::mesh_setup::{mesh, MeshAction};
use amp_adapter_gui::MultiplayGuiApp;
use std::time::{Duration, Instant};

fn settle(app: &mut MultiplayGuiApp) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.mesh_busy() {
        app.poll_mesh();
        assert!(Instant::now() < deadline, "joining never finished");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn off_by_default_and_not_searched() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    assert!(!app.config.p2p.enable_p2p_mesh);
    app.poll_mesh();
    assert!(!app.mesh_busy(), "nothing starts while off");
    assert!(!app.enabled_providers().contains(&"mesh"));
}

// One test drives the shared provider so joins do not race each other.
#[test]
fn joining_keeps_the_code_in_the_vault_and_a_bad_code_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    app.mesh_ui.local_only = true;
    app.config.p2p.enable_p2p_mesh = true;
    app.apply_mesh();
    settle(&mut app);
    assert!(
        app.mesh_status().contains("In the mesh"),
        "{}",
        app.mesh_status()
    );
    assert!(app.enabled_providers().contains(&"mesh"));
    let code = app.mesh_code().unwrap().to_string();

    let config_text: String = walk(dir.path())
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "toml" || e == "json"))
        .map(|p| std::fs::read_to_string(p).unwrap_or_default())
        .collect();
    assert!(
        !config_text.contains(&code),
        "the code is not in a settings file"
    );

    app.mesh_ui.code_input = "not a code".into();
    app.mesh_action(MeshAction::Join);
    assert!(
        app.mesh_status().contains("not a mesh code"),
        "{}",
        app.mesh_status()
    );
    assert_eq!(app.mesh_code(), Some(code.as_str()), "the old code stays");

    let theirs = amp_adapter_p2p::MeshCode::generate().to_string();
    app.mesh_ui.code_input = theirs.to_lowercase();
    app.mesh_action(MeshAction::Join);
    settle(&mut app);
    assert_eq!(app.mesh_code(), Some(theirs.as_str()));

    app.config.p2p.enable_p2p_mesh = false;
    app.mesh_action(MeshAction::Apply);
    assert!(!app.enabled_providers().contains(&"mesh"));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while rt.block_on(mesh().node()).is_some() {
        assert!(Instant::now() < deadline, "the node is left");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}
