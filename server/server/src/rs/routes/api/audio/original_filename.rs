use rocket::request::{FromRequest, Outcome, Request};

/// Request guard that extracts the optional X-Original-Filename header.
pub struct OriginalFilename(pub Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OriginalFilename {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let filename = request
            .headers()
            .get_one("X-Original-Filename")
            .map(|s| s.to_string());
        Outcome::Success(OriginalFilename(filename))
    }
}
