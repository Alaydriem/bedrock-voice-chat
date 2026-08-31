use common::curia;
use std::sync::Arc;

use common::response::admin::PeerLinkResponse;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::relay::PeerPlane;

/// This server's peer link, for the far side's `peer` block.
///
/// Served from the running endpoint rather than derived from `node.key`, because a
/// link carries the addresses a peer dials and those belong to the live socket.
///
/// The registry is asked what address it saw this server at, because a host behind
/// NAT observes only its LAN address and there is no relay to fall back on: an
/// address missing from the link is a path that does not exist. That observation is
/// best-effort — a link without it still reaches a same-host or LAN peer.
#[openapi(tag = "Admin")]
#[get("/relay/peerlink")]
pub async fn relay_peerlink(
    _admin: AdminGuard,
    plane: &State<Option<Arc<PeerPlane>>>,
    config: &State<crate::config::Server>,
) -> Result<Json<PeerLinkResponse>, Status> {
    let plane = plane.inner().as_ref().ok_or(Status::NotFound)?;

    let peerlink = plane
        .ticket_observed(crate::config::Registry::peerlink(), config.peer_port)
        .await
        .map_err(|e| {
            curia::error!("relay_peerlink: minting a ticket failed: {}", e);
            Status::InternalServerError
        })?;

    Ok(Json(PeerLinkResponse {
        peerlink,
        node_id: plane.node_id().to_string(),
    }))
}
