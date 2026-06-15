use std::sync::Arc;

use rocket::{http::Status, State};

use crate::relay::RegisterNonceStore;

// Endpoint-control proof responder: the relay's reachability callback hits this
// on the registrant's own HTTPS listener. We echo the nonce back ONLY when it
// is one we were actually issued (held in our `RegisterNonceStore`), proving we
// received the challenge for the endpoint we are claiming. A nonce we never
// received is rejected.
#[get("/proof/<nonce>")]
pub fn proof(nonce: &str, nonces: &State<Arc<RegisterNonceStore>>) -> Result<String, Status> {
    if nonces.contains(nonce) {
        Ok(nonce.to_string())
    } else {
        Err(Status::NotFound)
    }
}
