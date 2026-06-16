use std::sync::Arc;
use std::time::Instant;

use common::structs::relay::{LookupRequest, LookupResponse};
use rocket::{serde::json::Json, State};
use rocket_okapi::openapi;

use crate::relay::EndpointReachability;
use crate::relay::RelayRegistry;

// Lookup is gated on the same endpoint-control proof register requires: the
// caller must control the `caller` endpoint it claims. Without this an attacker
// who knows a world hash AND a registered member endpoint could pass
// `caller = victim_endpoint` to enumerate peers. We run the (idempotent)
// reachability proof for the token, then `lookup` enforces the token binding.
#[openapi(tag = "Relay")]
#[post("/lookup", data = "<payload>")]
pub async fn lookup(
    payload: Json<LookupRequest>,
    registry: &State<Arc<RelayRegistry>>,
    reachability: &State<Arc<dyn EndpointReachability>>,
) -> Json<LookupResponse> {
    let req = payload.0;
    let now = Instant::now();

    registry
        .verify_endpoint(&req.token, &req.caller, reachability.inner().as_ref(), now)
        .await;

    let worlds = registry
        .lookup(&req.caller, &req.hashed_worlds, &req.token, now)
        .await;
    Json(LookupResponse { worlds })
}
