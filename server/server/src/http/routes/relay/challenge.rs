use std::sync::Arc;
use std::time::Instant;

use common::structs::relay::{RegisterChallengeRequest, RegisterChallengeResponse};
use rocket::{serde::json::Json, State};

use crate::relay::RelayRegistry;

// Endpoint-control-proven registration, leg 1: issue a challenge bound to the
// claimed endpoint. The registrant must remember `nonce` so its own
// `/relay/proof/<nonce>` route can serve it when the relay's reachability
// callback fires during `register`. SPKI pinning authenticates the relay to
// callers; this gate authenticates the caller's endpoint claim to the relay.
#[post("/challenge", data = "<payload>")]
pub async fn challenge(
    payload: Json<RegisterChallengeRequest>,
    registry: &State<Arc<RelayRegistry>>,
) -> Json<RegisterChallengeResponse> {
    let req = payload.0;
    let (token, nonce) = registry.issue_challenge(&req.endpoint, Instant::now()).await;
    Json(RegisterChallengeResponse { token, nonce })
}
