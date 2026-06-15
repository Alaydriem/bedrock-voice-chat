use std::sync::Arc;

use common::structs::relay::{PeerCertRequest, PeerCertResponse};
use rocket::{http::Status, serde::json::Json, State};

use crate::services::relay::PeerCertIssueError;
use crate::services::PeerCertIssuer;

// Peer-cert issuance: the acceptor side issues the initiator an in-memory client
// cert for its `host:https_port` identity ONLY when that peer is mutually
// presence-proven for the shared world (default deny).
#[post("/peer-cert", data = "<payload>")]
pub fn peer_cert(
    payload: Json<PeerCertRequest>,
    issuer: &State<Arc<PeerCertIssuer>>,
) -> Result<Json<PeerCertResponse>, Status> {
    let req = payload.0;
    match issuer.issue(&req.host, req.port, &req.hashed_world) {
        Ok(resp) => Ok(Json(resp)),
        Err(PeerCertIssueError::NotProven { .. }) => Err(Status::Forbidden),
        Err(PeerCertIssueError::Signing(e)) => {
            tracing::error!("peer-cert issuance failed: {}", e);
            Err(Status::InternalServerError)
        }
    }
}
