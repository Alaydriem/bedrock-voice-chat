use rocket::request::{FromRequest, Outcome, Request};
use rocket_governor::{RocketGovernable, RocketGovernor};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

// A rate-limit guard the OpenAPI generator can see.
//
// `RocketGovernor` does not implement `OpenApiFromRequest`, and the orphan rule
// prevents adding it here, so a route that takes one directly cannot carry
// `#[openapi]` — it compiles, mounts, serves traffic, and is absent from the
// published spec with no warning. Wrapping it restores the annotation without
// changing the limiting behaviour.
//
// The guard reads only the client address, so it contributes no request
// parameters of its own.
pub struct RateLimited<'r, T: RocketGovernable<'r>>(pub RocketGovernor<'r, T>);

#[rocket::async_trait]
impl<'r, T: RocketGovernable<'r>> FromRequest<'r> for RateLimited<'r, T> {
    type Error = <RocketGovernor<'r, T> as FromRequest<'r>>::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        RocketGovernor::from_request(request).await.map(Self)
    }
}

impl<'r, T: RocketGovernable<'r>> OpenApiFromRequest<'r> for RateLimited<'r, T> {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}
