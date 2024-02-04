use rocket::{ mtls::Certificate, http::Status };

#[get("/ping")]
pub async fn pong<'r>(_identity: Certificate<'r>) -> Status {
    return Status::Ok;
}
