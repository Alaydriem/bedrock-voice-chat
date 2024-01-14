use rocket::{ mtls::Certificate, http::Status };

#[get("/ping")]
pub async fn pong<'r>(identify: Certificate<'r>) -> Status {
    return Status::Ok;
}
