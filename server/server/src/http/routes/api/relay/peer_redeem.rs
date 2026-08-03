use std::sync::Arc;
use std::time::Instant;

use common::structs::relay::{PeerCertResponse, PeerRedeemRequest};
use rocket::{State, http::Status, serde::json::Json};

use crate::http::guards::{RateLimited, RelayRedeemRateLimit};
use crate::relay::{RedeemError, ServerPeerStore};
use rocket_okapi::openapi;

/// Redeem a peer code observed through the realm for the in-memory peer cert.
///
/// Single-use and recipient-bound: the presenter endpoint must match the code's
/// bound recipient.
#[openapi(tag = "Relay")]
#[post("/peer-redeem", data = "<payload>")]
pub fn peer_redeem(
    _rate_limit: RateLimited<'_, RelayRedeemRateLimit>,
    payload: Json<PeerRedeemRequest>,
    store: &State<Arc<ServerPeerStore>>,
) -> Result<Json<PeerCertResponse>, Status> {
    let req = payload.0;
    let presenter = format!("{}:{}", req.presenter_host, req.presenter_port);
    match store.redeem(&req.code, &presenter, Instant::now()) {
        Ok(id) => Ok(Json(PeerCertResponse {
            ca_pem: id.ca_pem,
            cert_pem: id.cert_pem,
            key_pem: id.key_pem,
        })),
        Err(RedeemError::NotFound) => Err(Status::NotFound),
        Err(RedeemError::Expired) => Err(Status::Gone),
        Err(RedeemError::AlreadyUsed) | Err(RedeemError::WrongRecipient) => Err(Status::Forbidden),
        Err(RedeemError::AtCapacity) => Err(Status::ServiceUnavailable),
    }
}
