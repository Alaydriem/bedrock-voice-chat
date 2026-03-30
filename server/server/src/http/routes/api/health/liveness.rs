use rocket::http::Status;
use rocket_okapi::openapi;

#[openapi(tag = "Health")]
#[get("/liveness")]
pub async fn liveness() -> Status {
    Status::Ok
}
