use std::sync::Arc;

use rocket::State;
use rocket::http::{ContentType, Status};
use rocket_okapi::openapi;

use crate::services::CurrentNonce;

/// Echoes the challenge the relay registry most recently sent over the enrollment
/// session.
///
/// Unauthenticated by necessity: the relay fetches this from the address the operator
/// declared, before it holds any credential for this server, and the value is a
/// single-use random string that reveals nothing. Answering it is what binds the
/// published address record to this server's node key.
///
/// 404 when no challenge has arrived. An empty 200 would read to the relay as a
/// server answering with the wrong value rather than one that has not been asked.
#[openapi(tag = "Health")]
#[get("/enrollment-nonce")]
pub async fn enrollment_nonce(
    nonce: &State<Arc<CurrentNonce>>,
) -> Result<(ContentType, String), Status> {
    match nonce.get() {
        Some(value) => Ok((ContentType::Plain, value)),
        None => Err(Status::NotFound),
    }
}
