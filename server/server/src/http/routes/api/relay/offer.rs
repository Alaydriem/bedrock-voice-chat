use std::sync::Arc;
use std::time::{Duration, Instant};

use common::structs::packet::PeerPresenceInjectPacket;
use common::structs::relay::OfferRequest;
use rocket::{State, http::Status, serde::json::Json};

use crate::http::guards::{RateLimited, RelayOfferRateLimit};
use crate::relay::{CodeSealer, LocalInjectDelivery, ServerPeerStore};
use rocket_okapi::openapi;

// How long a freshly offered peer code stays redeemable.
const OFFER_CODE_TTL: Duration = Duration::from_secs(180);

/// Mint a single-use, recipient-bound peer code.
///
/// Scoped to `hashed_world` and sealed to the asker's public key, then injected
/// into the realm via this server's own client. Only a live member of the world
/// observes it, and only the bound asker can both unseal and redeem it.
///
/// Returns 202: the sealed code travels via the realm, never the HTTP response.
#[openapi(tag = "Relay")]
#[post("/offer", data = "<payload>")]
pub fn offer(
    _rate_limit: RateLimited<'_, RelayOfferRateLimit>,
    payload: Json<OfferRequest>,
    store: &State<Arc<ServerPeerStore>>,
    inject: &State<Arc<dyn LocalInjectDelivery>>,
) -> Status {
    let req = payload.0;
    let code = match store.mint(
        &req.hashed_world,
        &req.asker_host,
        req.asker_port,
        OFFER_CODE_TTL,
        Instant::now(),
    ) {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("relay offer: code mint failed: {}", e);
            return Status::InternalServerError;
        }
    };

    // Seal the code to the asker's public key so an observer in the realm cannot
    // read (let alone redeem) it.
    let sealed = match CodeSealer::seal(&code, &req.asker_public_key) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("relay offer: sealing code failed: {}", e);
            return Status::BadRequest;
        }
    };

    inject.deliver_inject(
        &req.hashed_world,
        PeerPresenceInjectPacket {
            token: sealed,
            ttl_ms: OFFER_CODE_TTL.as_millis() as u32,
        },
    );
    Status::Accepted
}
