use rocket::http::Status;
use rocket::Request;

pub struct DefaultCatcher;

impl DefaultCatcher {
    pub fn status_string(status: Status) -> String {
        format!("{} {}", status.code, status.reason_lossy())
    }
}

#[rocket::catch(default)]
pub fn default_catcher(status: Status, _request: &Request<'_>) -> String {
    DefaultCatcher::status_string(status)
}
