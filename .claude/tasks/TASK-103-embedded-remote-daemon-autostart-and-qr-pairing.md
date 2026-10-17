# TASK-103: Embedded Remote Daemon Auto-Start & QR Code Pairing in GUI

## Plan Reference
- Plan: [PLAN-032](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-032-integrated-mobile-remote-daemon-and-distributed-jam-session.md)
- Role: `distributed_systems` & `frontend_gui`

## Objectives
1. Add remote pairing configuration and state to `MultiplayGuiApp` (`RemoteState`).
2. Implement `QrCodeWidget` rendering the boolean module matrix from `QrPairingRenderer::render_matrix` directly into egui Painter with crisp dark/light tiles.
3. Add a dedicated "Remote Control" tab or satellite window with live LAN IP address, port, ephemeral pairing token, and QR code.
4. Auto-spawn `RemoteDaemon` background server on an available or configured LAN port (default 9002).

## Verification Command
`cargo test -p amp-adapter-remote -p amp-adapter-gui`

## Status
- **Status:** Completed
