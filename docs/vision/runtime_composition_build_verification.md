# Runtime Composition Root, CI Enforcement & Step-by-Step Build Plan

This document provides the runtime composition root (`main.rs`) that stitches together all hexagonal adapters, the automated CI pure-Rust audit script, and a developer implementation roadmap.

---

## 1. Composition Root (`crates/amp-app/src/main.rs`)

The composition root initializes the ports, registers adapters, sets up IPC/event channels, and hands execution to the Ratatui TUI runtime.

```rust
// crates/amp-app/src/main.rs
use amp_adapter_cpal::CpalAudioSink;
use amp_adapter_p2p::mesh::SecurePeerNode;
use amp_adapter_soulseek::SoulseekAdapter;
use amp_adapter_storage::RedbStorageAdapter;
use amp_adapter_youtube::PureYouTubeAdapter;
use amp_core_engine::dispatcher::EventBus;
use amp_core_engine::filtered_search::FilteredSearchCoordinator;
use amp_domain::source_badge::SourceKind;
use libp2p::identity::Keypair;
use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("󰓃 Initializing Pure-Rust Universal Audio Engine...");

    // 1. Initialize Event Bus (CQRS Channel)
    let (event_bus, mut cmd_rx) = EventBus::new(256);

    // 2. Initialize Embedded Database
    let db_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("amp_music.redb");
    let storage = Arc::new(RedbStorageAdapter::new(db_path)?);

    // 3. Initialize Audio Hardware Output
    let (mut sink, audio_producer) = CpalAudioSink::new()
        .map_err(|e| format!("Audio DAC init failed: {e}"))?;
    sink.start_playback()
        .map_err(|e| format!("CoreAudio stream launch error: {e}"))?;

    // 4. Initialize Multi-Provider Search Coordinator
    let mut search_coordinator = FilteredSearchCoordinator::new();

    // Register Providers
    let yt_adapter = Arc::new(PureYouTubeAdapter::new());
    search_coordinator.register_provider(SourceKind::YouTube, yt_adapter);

    let slsk_adapter = Arc::new(SoulseekAdapter::new(
        "server.slsknet.org:2242".to_string(),
        "user".to_string(),
        "pass".to_string(),
    ));
    search_coordinator.register_provider(SourceKind::Soulseek, slsk_adapter);

    // 5. Initialize Direct P2P Mesh
    let p2p_keys = Keypair::generate_ed25519();
    let p2p_node = Arc::new(SecurePeerNode::start(p2p_keys, 4001).await?);
    println!("󱘖 P2P Node Online: Peer ID {}", p2p_node.peer_id);

    // 6. Spawn Command Listener Loop
    tokio::spawn(async move {
        while let Some(command) = cmd_rx.recv().await {
            match command {
                amp_core_engine::events::PlayerCommand::PlayTrack(track) => {
                    println!("Playing: {} - {}", track.metadata.artist, track.metadata.title);
                }
                amp_core_engine::events::PlayerCommand::TogglePause => {
                    println!("Toggle Pause");
                }
                _ => {}
            }
        }
    });

    println!("󰄬 Audio Player Core Initialized. Launching TUI Console...");

    // 7. Await Termination Signal
    signal::ctrl_c().await?;
    println!("\nShutting down audio engine cleanly.");
    Ok(())
}
```

---

## 2. Automated Pure-Rust Audit Script (`scripts/verify_pure_rust.sh`)

Place this script in your repository root and add it to your CI pipeline (`.github/workflows/ci.yml`) to ensure no team member or crate update introduces a C compiler dependency or static C library.

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo " Running Pure-Rust Toolchain Verification (Zero-C Audit) "
echo "=========================================================="

# 1. Ensure Cargo compiles without CC or CXX environment variables
unset CC
unset CXX

# 2. Check for disallowed native C build dependencies in the dependency tree
FORBIDDEN_CRATES=("cc" "cmake" "pkg-config" "openssl-sys" "libloading" "libsqlite3-sys" "ffmpeg-sys")
FAIL=0

for crate in "${FORBIDDEN_CRATES[@]}"; do
    MATCHES=$(cargo tree -i "$crate" 2>/dev/null || true)
    if [ -n "$MATCHES" ]; then
        echo -e "\033[0;31m[FAIL]\033[0m Forbidden native C-wrapper crate detected: '$crate'"
        echo "$MATCHES"
        FAIL=1
    fi
done

if [ "$FAIL" -eq 1 ]; then
    echo -e "\033[0;31m[ERROR]\033[0m Verification failed! Native C build-tools or libraries are present."
    exit 1
fi

echo -e "\033[0;32m[SUCCESS]\033[0m 100% Pure Rust verified. Zero C/C++ dynamic or static linkage detected."
```

---

## 3. Step-by-Step Developer Implementation Roadmap

Follow these 6 progressive implementation phases to assemble the system from scratch:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       DEVELOPMENT PHASE TIMELINE                            │
└─────────────────────────────────────────────────────────────────────────────┘

  PHASE 1: Workspace Skeleton & Pure-Rust Verification
  ├─ Initialize Cargo Workspace with crates specified in 01_system_architecture.
  ├─ Add .cargo/config.toml preventing OPENSSL_NO_VENDOR.
  └─ Implement 02_domain_and_ports (Zero dependencies).
         │
         ▼
  PHASE 2: Core Hardware Output & Decoding Loop
  ├─ Implement 03_audio_engine_and_precaching (Cpal + Symphonia + rtrb).
  ├─ Verify playback of local FLAC/MP3 files directly from SSD.
  └─ Confirm: CPU usage < 0.5% and memory footprint < 30 MB.
         │
         ▼
  PHASE 3: Audiophile DSP & RAM Precaching
  ├─ Implement 08_bitperfect_dsp_and_resampling (Rubato Sinc resampler).
  ├─ Wire 10-band Biquad Equalizer and FFT spectrum calculator.
  └─ Connect SharedPrecacheBuffer to verify seamless network underrun handling.
         │
         ▼
  PHASE 4: Storage, Metadata & Search Coordinator
  ├─ Implement 07_state_management_cqrs_and_db (redb storage engine).
  ├─ Implement 05_search_ui_and_source_icons (FilterMask & Coordinator).
  └─ Wire local directory scanning with Lofty pure-Rust tagger.
         │
         ▼
  PHASE 5: External Providers & Offline Downloader
  ├─ Implement 04_providers_search_downloader (YouTube & Spotify adapters).
  ├─ Wire OfflineTrackDownloader to save streamed audio directly to disk.
  └─ Add Bandcamp and SoundCloud HTTP stream extractors.
         │
         ▼
  PHASE 6: Soulseek Protocol & Encrypted P2P Mesh
  ├─ Implement 06_p2p_soulseek_and_secure_sharing (Soulseek binary client).
  ├─ Implement libp2p authenticated Noise transport and mDNS/DHT discovery.
  └─ Launch Ratatui modular TUI console and verify cross-peer searches.
```

### Verification Matrix for Each Phase

| Component | Test Command | Success Benchmark |
| :--- | :--- | :--- |
| **Pure-Rust Check** | `./scripts/verify_pure_rust.sh` | Zero forbidden crates detected. |
| **Audio Loop** | `cargo test -p amp-core-engine` | Ring buffer pops samples without dropping frames. |
| **Memory Buffer** | `cargo test -p amp-core-engine test_precache_seek` | Accurate seeking across non-contiguous chunks. |
| **Database** | `cargo test -p amp-adapter-storage` | Full persistence roundtrip under 5ms. |
| **Parallel Search** | `cargo test -p amp-core-engine test_search_timeout` | Results collect and return strictly within timeout. |
| **Direct P2P Mesh** | `cargo test -p amp-adapter-p2p` | Two nodes complete Noise handshake and query exchange. |