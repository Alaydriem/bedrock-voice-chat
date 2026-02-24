use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};

use super::HytaleSessionIdError;

#[derive(Clone, Debug)]
pub struct HytaleSessionId(pub String);

#[async_trait]
impl<'r> FromRequest<'r> for HytaleSessionId {
    type Error = HytaleSessionIdError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("X-Session-Id") {
            Some(session_id) => Outcome::Success(HytaleSessionId(session_id.to_string())),
            None => Outcome::Error((Status::BadRequest, HytaleSessionIdError::Missing)),
        }
    }
}
