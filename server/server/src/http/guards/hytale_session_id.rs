use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

#[derive(Clone, Debug)]
pub struct HytaleSessionId(pub String);

#[derive(Debug)]
pub enum HytaleSessionIdError {
    Missing,
}

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

impl<'a> OpenApiFromRequest<'a> for HytaleSessionId {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = okapi::openapi3::SecurityScheme {
            description: Some("Hytale device flow session identifier".into()),
            data: okapi::openapi3::SecuritySchemeData::ApiKey {
                name: "X-Session-Id".into(),
                location: "header".into(),
            },
            extensions: Default::default(),
        };
        let mut security_req = okapi::openapi3::SecurityRequirement::new();
        security_req.insert("HytaleSessionId".into(), Vec::new());
        Ok(RequestHeaderInput::Security(
            "HytaleSessionId".into(),
            security_scheme,
            security_req,
        ))
    }
}
