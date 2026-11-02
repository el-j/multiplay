//! Finding mesh members on the local network with mDNS (`_multiplay._tcp.local.`). The
//! announcement carries only the peer id and the mesh's discovery id (a hash, not the code);
//! members of other meshes ignore each other, and the pnet handshake refuses them anyway.

#![forbid(unsafe_code)]

use crate::node::{MeshConfig, MeshNode};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

const SERVICE: &str = "_multiplay._tcp.local.";

/// Announces `node` and dials members of the same mesh as they appear.
pub(crate) async fn start(node: &MeshNode, config: &MeshConfig) {
    let Some(port) = node.listen_addrs().await.iter().find_map(|a| {
        a.iter().find_map(|p| match p {
            libp2p::multiaddr::Protocol::Tcp(port) => Some(port),
            _ => None,
        })
    }) else {
        return;
    };
    let mesh = config.code.discovery_id();
    let me = node.peer_id();
    let tx = node.dial_tx();
    let spawned = std::thread::Builder::new()
        .name("multiplay-mesh-mdns".into())
        .spawn(move || {
            let Ok(daemon) = ServiceDaemon::new() else {
                tracing::warn!("mDNS unavailable: mesh members must be added by address");
                return;
            };
            let host = format!("multiplay-{}.local.", &me[me.len().saturating_sub(8)..]);
            let props = [("peer", me.as_str()), ("mesh", mesh.as_str())];
            if let Ok(info) = ServiceInfo::new(SERVICE, &me, &host, "", port, &props[..])
                .map(ServiceInfo::enable_addr_auto)
            {
                let _ = daemon.register(info);
            }
            let Ok(events) = daemon.browse(SERVICE) else {
                return;
            };
            while let Ok(event) = events.recv() {
                if tx.is_closed() {
                    break;
                }
                let ServiceEvent::ServiceResolved(found) = event else {
                    continue;
                };
                let (Some(peer), Some(their_mesh)) = (
                    found.get_property_val_str("peer"),
                    found.get_property_val_str("mesh"),
                ) else {
                    continue;
                };
                if their_mesh != mesh || peer == me {
                    continue;
                }
                for ip in found.get_addresses_v4() {
                    if let Ok(addr) =
                        format!("/ip4/{ip}/tcp/{}/p2p/{peer}", found.get_port()).parse()
                    {
                        let _ = tx.send(crate::node::dial_command(addr));
                    }
                }
            }
            let _ = daemon.shutdown();
        });
    if spawned.is_err() {
        tracing::warn!("mesh discovery could not start");
    }
}
