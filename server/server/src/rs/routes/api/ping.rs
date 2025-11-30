use rocket::{http::Status, mtls::Certificate};

#[get("/ping")]
pub async fn pong<'r>(_identity: Certificate<'r>) -> Status {
    return Status::Ok;
}
