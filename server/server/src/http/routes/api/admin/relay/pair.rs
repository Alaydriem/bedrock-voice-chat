use std::sync::Arc;
use std::time::Duration;

use common::curia;
use common::request::admin::PairingRequest;
use common::response::admin::{PairedPeer, PairedPeersResponse, PairingCodeResponse};
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::relay::PeerPlane;
use crate::services::PairingService;

/// Mint a single-use pairing code for a voice bridge.
///
/// The plaintext is returned to the caller and stored nowhere. It is not logged here or
/// anywhere else: a code in a log file is a credential in a log file.
#[openapi(tag = "Admin")]
#[post("/relay/pair", data = "<request>")]
pub async fn relay_pair(
    _admin: AdminGuard,
    db: Db<'_>,
    request: Json<PairingRequest>,
) -> Result<Json<PairingCodeResponse>, Status> {
    let conn = db.into_inner();
    let ttl = Duration::from_secs(
        request
            .ttl_secs
            .unwrap_or(PairingService::DEFAULT_TTL.as_secs()),
    );

    let code = PairingService::mint(conn, &request.label, ttl)
        .await
        .map_err(|e| {
            curia::error!("relay_pair: minting a pairing code failed: {}", e);
            Status::InternalServerError
        })?;

    Ok(Json(PairingCodeResponse {
        code,
        label: request.label.clone(),
        expires_in_secs: ttl.as_secs(),
    }))
}

/// List the bridges that have redeemed a pairing code.
///
/// Peers declared in `config.hcl` are absent: they are already visible in the file an
/// operator wrote, and listing them here would suggest they could be revoked from here.
#[openapi(tag = "Admin")]
#[get("/relay/paired")]
pub async fn relay_paired(
    _admin: AdminGuard,
    db: Db<'_>,
) -> Result<Json<PairedPeersResponse>, Status> {
    let conn = db.into_inner();

    let rows = PairingService::paired(conn).await.map_err(|e| {
        curia::error!("relay_paired: listing paired peers failed: {}", e);
        Status::InternalServerError
    })?;

    Ok(Json(PairedPeersResponse {
        peers: rows
            .into_iter()
            .map(|row| PairedPeer {
                node_id: row.node_id,
                label: row.label,
                paired_at: row.paired_at,
            })
            .collect(),
    }))
}

/// Revoke every grant carrying this label.
///
/// The running grant table is updated as well as the row, so the peer's next connection is
/// refused rather than its existing authorization surviving until a restart.
#[openapi(tag = "Admin")]
#[delete("/relay/paired/<label>")]
pub async fn relay_unpair(
    _admin: AdminGuard,
    db: Db<'_>,
    plane: &State<Option<Arc<PeerPlane>>>,
    label: &str,
) -> Result<Json<PairedPeersResponse>, Status> {
    let conn = db.into_inner();

    PairingService::revoke(conn, label).await.map_err(|e| {
        curia::error!("relay_unpair: revoking a paired peer failed: {}", e);
        Status::InternalServerError
    })?;

    // Reached through the plane rather than as its own managed state: the plane already
    // holds the table it authorizes against, and a second handle could drift from it.
    if let Some(plane) = plane.inner() {
        plane.grants().forget(label);
    }

    let rows = PairingService::paired(conn).await.map_err(|e| {
        curia::error!("relay_unpair: listing paired peers failed: {}", e);
        Status::InternalServerError
    })?;

    Ok(Json(PairedPeersResponse {
        peers: rows
            .into_iter()
            .map(|row| PairedPeer {
                node_id: row.node_id,
                label: row.label,
                paired_at: row.paired_at,
            })
            .collect(),
    }))
}
