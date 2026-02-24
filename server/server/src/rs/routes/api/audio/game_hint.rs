use rocket::request::{FromRequest, Outcome, Request};

/// Request guard that extracts the optional X-Game header for legacy cert disambiguation.
pub struct GameHint(pub Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GameHint {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let game = request
            .headers()
            .get_one("X-Game")
            .map(|s| s.to_string());
        Outcome::Success(GameHint(game))
    }
}
