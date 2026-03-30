use rocket::{http::Status, mtls::Certificate};
use rocket_okapi::openapi;

#[openapi(tag = "Health")]
#[get("/ping")]
pub async fn pong(_identity: Certificate<'_>) -> Status {
    return Status::Ok;
}
