use rocket::http::Status;
use rocket_okapi::openapi;

/// Liveness probe. Reports that the process is running.
///
/// Unauthenticated, for use as a container restart check.
#[openapi(tag = "Health")]
#[get("/liveness")]
pub async fn liveness() -> Status {
    Status::Ok
}
