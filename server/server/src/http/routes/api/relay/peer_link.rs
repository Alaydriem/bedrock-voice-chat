use std::sync::Arc;
use std::time::{Duration, Instant};

use common::structs::permission::Permission;
use common::structs::relay::{PeerLinkRequest, PeerLinkResponse};
use rocket::{State, http::Status, mtls::Certificate, serde::json::Json};
use rocket_okapi::openapi;

use crate::config::Permissions;
use crate::http::pool::Db;
use crate::relay::ServerPeerStore;
use crate::services::{AuthService, PermissionService};

// How long a Flow-2 peer-link code stays redeemable.
const PEER_LINK_CODE_TTL: Duration = Duration::from_secs(180);

// Flow 2 (designated peer / operator): an mTLS-authenticated player who holds the
// `PeerLink` permission requests a peer-link code for `hashed_world`, bound to the
// endpoint it will dial from. Authorization is the player's granted permission —
// not merely "has a cert" — so an authenticated player WITHOUT the grant is
// refused (403). Unlike Flow 1, the code is returned directly (the grant is
// proven over mTLS) rather than injected through the realm. The code is
// single-use, recipient-bound, world-scoped, and short-TTL; the caller redeems it
// at `/relay/peer-redeem` for a `server::`-CN peer cert.
#[openapi(tag = "Relay")]
#[post("/peer-link", data = "<payload>")]
pub async fn peer_link(
    identity: Certificate<'_>,
    db: Db<'_>,
    perm_config: &State<Permissions>,
    store: &State<Arc<ServerPeerStore>>,
    payload: Json<PeerLinkRequest>,
) -> Result<Json<PeerLinkResponse>, Status> {
    let conn = db.into_inner();

    let player = match AuthService::player_from_certificate(&identity, conn, None).await {
        Ok(p) => p,
        Err(status) => return Err(status),
    };

    let perm_service = PermissionService::new(perm_config.defaults.clone());
    if !perm_service
        .evaluate(conn, player.id, &Permission::PeerLink)
        .await
    {
        return Err(Status::Forbidden);
    }

    let req = payload.0;
    match store.mint(
        &req.hashed_world,
        &req.recipient_host,
        req.recipient_port,
        PEER_LINK_CODE_TTL,
        Instant::now(),
    ) {
        Ok(code) => Ok(Json(PeerLinkResponse { code })),
        Err(e) => {
            tracing::error!("relay peer-link: code mint failed: {}", e);
            Err(Status::ServiceUnavailable)
        }
    }
}
