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

/// Request a peer-link code as a designated peer or operator.
///
/// Requires an mTLS-authenticated player holding the `peer_link` permission. The
/// grant itself is the authorization, not merely holding a certificate, so an
/// authenticated player without it is refused with 403.
///
/// The code is returned directly here, rather than injected through the realm,
/// because the grant is proven over mTLS. It is single-use, recipient-bound,
/// world-scoped and short-lived; redeem it at `/api/relay/peer-redeem` for a peer
/// certificate.
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
