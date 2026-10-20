//! One remote socket: plain-HTTP fallback, the authenticated WebSocket upgrade, and the message
//! loop that routes transport actions to the engine and requests to the host.

#![forbid(unsafe_code)]

use crate::devices::DeviceId;
use crate::protocol::{self, RemoteAction, RemoteEventBroadcast};
use crate::reference::{self, TrackReference};
use crate::requests::{HostRequest, RequestStatus, TrackRequest};
use crate::server::{authorize, lock, Granted, Outbound, RemoteDaemon, Shared};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

// tungstenite's handshake callback signature fixes the large `ErrorResponse` error type.
#[allow(clippy::result_large_err)]
pub(crate) async fn handle_connection(
    shared: Arc<Shared>,
    mut stream: TcpStream,
    peer: SocketAddr,
) {
    let timeout = shared.config.handshake_timeout;
    let mut peek_buf = [0u8; 2048];
    let peeked = match tokio::time::timeout(timeout, stream.peek(&mut peek_buf)).await {
        Ok(Ok(n)) => n,
        _ => return,
    };
    let head = String::from_utf8_lossy(&peek_buf[..peeked]);
    if head.starts_with("POST /api/upload ") {
        crate::upload::handle(&shared, stream, peer).await;
        return;
    }
    if !head.to_ascii_lowercase().contains("upgrade: websocket") {
        let response = crate::http::respond(head.lines().next().unwrap_or(""));
        // Drain the peeked request so closing the socket does not send a TCP reset.
        let mut drain = [0u8; 2048];
        let _ = tokio::time::timeout(timeout, stream.read(&mut drain[..peeked])).await;
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
        return;
    }

    let mut granted = None;
    let callback = |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
        match authorize(&shared, peer.ip(), req) {
            Ok(g) => {
                granted = Some(g);
                Ok(resp)
            }
            Err(rejection) => Err(rejection.into_response()),
        }
    };
    let ws_config = WebSocketConfig::default()
        .max_message_size(Some(shared.config.max_message_bytes))
        .max_frame_size(Some(shared.config.max_message_bytes));
    let accept = tokio_tungstenite::accept_hdr_async_with_config(stream, callback, Some(ws_config));
    let (ws, granted) = match tokio::time::timeout(timeout, accept).await {
        Ok(Ok(ws)) => match granted {
            Some(g) => (ws, g),
            None => return,
        },
        _ => return,
    };

    let device = granted.device().clone();
    shared.active_peers.fetch_add(1, Ordering::SeqCst);
    *lock(&shared.connected).entry(device.clone()).or_default() += 1;
    lock(&shared.auth).sessions.touch(&device);
    serve(&shared, ws, &device, granted).await;
    lock(&shared.auth).sessions.touch(&device);
    if let Some(n) = lock(&shared.connected).get_mut(&device) {
        *n = n.saturating_sub(1);
    }
    shared.active_peers.fetch_sub(1, Ordering::SeqCst);
}

async fn serve(
    shared: &Shared,
    ws: tokio_tungstenite::WebSocketStream<TcpStream>,
    device: &DeviceId,
    granted: Granted,
) {
    let (mut tx, mut rx) = ws.split();
    let mut outbound_rx = shared.outbound.subscribe();
    let mut shutdown_rx = shared.shutdown_tx.subscribe();

    if let Granted::NewSession(_, session) = granted {
        let msg =
            protocol::encode_server_message(&RemoteEventBroadcast::SessionGranted { session });
        if tx.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }
    if shared.uploads_available() {
        let msg = protocol::encode_server_message(&RemoteEventBroadcast::Uploads { enabled: true });
        if tx.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }

    loop {
        let reply = tokio::select! {
            incoming = rx.next() => match incoming {
                Some(Ok(Message::Text(text))) => match protocol::parse_client_message(&text) {
                    Ok(action) => handle_action(shared, device, action),
                    Err(e) => Some(RemoteEventBroadcast::Error { message: e.to_string() }),
                },
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => None,
            },
            event = outbound_rx.recv() => match event {
                Ok(Outbound::All(event)) => Some(event),
                Ok(Outbound::To(to, event)) if &to == device => Some(event),
                Ok(Outbound::Kick(None)) => break,
                Ok(Outbound::Kick(Some(to))) if &to == device => break,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => None,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            _ = shutdown_rx.changed() => break,
        };
        if let Some(event) = reply {
            let text = protocol::encode_server_message(&event);
            if tx.send(Message::Text(text.into())).await.is_err() {
                return;
            }
        }
    }
    let _ = tx.send(Message::Close(None)).await;
}

/// Applies one parsed action; returns the reply for this socket, if any.
fn handle_action(
    shared: &Shared,
    device: &DeviceId,
    action: RemoteAction,
) -> Option<RemoteEventBroadcast> {
    match action {
        RemoteAction::EnqueueById(id) => request_track(shared, device, TrackReference::Library(id)),
        RemoteAction::RequestTrack(reference) => request_track(shared, device, reference),
        RemoteAction::Search(query) => search(shared, device, &query),
        transport => {
            RemoteDaemon::dispatch_action(transport, &shared.event_bus);
            None
        }
    }
}

fn refused(reason: &str) -> Option<RemoteEventBroadcast> {
    Some(RemoteEventBroadcast::RequestRefused {
        reason: reason.to_string(),
        search: false,
    })
}

fn search_refused(reason: &str) -> Option<RemoteEventBroadcast> {
    Some(RemoteEventBroadcast::RequestRefused {
        reason: reason.to_string(),
        search: true,
    })
}

fn request_track(
    shared: &Shared,
    device: &DeviceId,
    reference: TrackReference,
) -> Option<RemoteEventBroadcast> {
    let reference = match reference.validate() {
        Ok(r) => r,
        Err(e) => return refused(e.0),
    };
    let Some((device_name, trusted)) = lock(&shared.auth).sessions.describe(device) else {
        return refused("This device is no longer paired.");
    };
    let id = match lock(&shared.requests).open(device, Instant::now()) {
        Ok(id) => id,
        Err(reason) => return refused(reason),
    };
    let request = TrackRequest {
        id,
        device: device.clone(),
        device_name,
        trusted,
        reference,
    };
    if shared.host_tx.send(HostRequest::Track(request)).is_err() {
        lock(&shared.requests).update(id, RequestStatus::Rejected);
        return refused("The host is not taking requests right now.");
    }
    Some(RemoteEventBroadcast::RequestUpdate {
        id,
        status: RequestStatus::Pending,
        title: None,
    })
}

fn search(shared: &Shared, device: &DeviceId, query: &str) -> Option<RemoteEventBroadcast> {
    let query = match reference::validate_query(query) {
        Ok(q) => q,
        Err(e) => return search_refused(e.0),
    };
    if let Err(reason) = lock(&shared.requests).allow_search(device, Instant::now()) {
        return search_refused(reason);
    }
    let request = HostRequest::Search {
        device: device.clone(),
        query,
    };
    if shared.host_tx.send(request).is_err() {
        return search_refused("The host is not taking requests right now.");
    }
    None
}
