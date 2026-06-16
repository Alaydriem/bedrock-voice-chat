use std::sync::Arc;
use std::time::Instant;

use common::structs::relay::RegisterRequest;
use rocket::{http::Status, serde::json::Json, State};
use rocket_okapi::openapi;

use crate::relay::EndpointReachability;
use crate::relay::RelayRegistry;

// Public TLS, no client-cert guard: callers authenticate the relay via SPKI
// pinning (RelayClient). Registration is gated on endpoint-control proof: the
// token must be live and bound to the claimed endpoint, and the relay confirms
// control by fetching its nonce back from that endpoint (reachability callback)
// before accepting. Lookups are later scoped to worlds the caller registered.
#[openapi(tag = "Relay")]
#[post("/register", data = "<payload>")]
pub async fn register(
    payload: Json<RegisterRequest>,
    registry: &State<Arc<RelayRegistry>>,
    reachability: &State<Arc<dyn EndpointReachability>>,
) -> Status {
    let req = payload.0;
    let now = Instant::now();

    // Prove the registrant controls the endpoint it claims before trusting the
    // token (idempotent: a no-op once the challenge is already verified).
    registry
        .verify_endpoint(&req.token, &req.endpoint, reachability.inner().as_ref(), now)
        .await;

    let ok = registry
        .register(&req.hashed_world, req.endpoint, req.ttl_secs, &req.token, now)
        .await;
    if ok {
        Status::NoContent
    } else {
        Status::Forbidden
    }
}
