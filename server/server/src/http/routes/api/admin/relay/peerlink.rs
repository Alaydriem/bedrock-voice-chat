use common::curia;
use std::sync::Arc;

use common::response::admin::PeerLinkResponse;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::relay::PeerPlane;

/// This server's peer link, for the far side's `peer` block.
///
/// Served from the running endpoint rather than derived from `node.key`, because
/// a link carries the addresses a peer dials and those belong to the live socket.
/// A link minted anywhere else would advertise a port nothing is listening on,
/// and the direct path — the only path a same-host peer has — would never come
/// up.
#[openapi(tag = "Admin")]
#[get("/relay/peerlink")]
pub async fn relay_peerlink(
    _admin: AdminGuard,
    plane: &State<Option<Arc<PeerPlane>>>,
) -> Result<Json<PeerLinkResponse>, Status> {
    let plane = plane.inner().as_ref().ok_or(Status::NotFound)?;

    let peerlink = plane.endpoint().ticket().await.map_err(|e| {
        curia::error!("relay_peerlink: minting a ticket failed: {}", e);
        Status::InternalServerError
    })?;

    Ok(Json(PeerLinkResponse {
        peerlink,
        node_id: plane.node_id().to_string(),
    }))
}
