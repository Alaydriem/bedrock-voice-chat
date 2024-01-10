use rocket::{ mtls::Certificate, http::Status, State };

#[get("/ping")]
pub async fn pong<'r>(identify: Certificate<'r>) -> Status {
    return Status::Ok;
}
