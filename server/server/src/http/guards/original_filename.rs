use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

pub struct OriginalFilename(pub String);

#[async_trait]
impl<'r> FromRequest<'r> for OriginalFilename {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("X-Original-Filename") {
            Some(filename) => Outcome::Success(OriginalFilename(filename.to_string())),
            None => Outcome::Forward(Status::Ok),
        }
    }
}

impl<'a> OpenApiFromRequest<'a> for OriginalFilename {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let parameter = okapi::openapi3::Parameter {
            name: "X-Original-Filename".into(),
            location: "header".into(),
            description: Some("Original filename of the uploaded audio file".into()),
            required: false,
            deprecated: false,
            allow_empty_value: false,
            value: okapi::openapi3::ParameterValue::Schema {
                style: None,
                explode: None,
                allow_reserved: false,
                schema: schemars::schema_for!(String).schema.into(),
                example: None,
                examples: None,
            },
            extensions: Default::default(),
        };
        Ok(RequestHeaderInput::Parameter(parameter))
    }
}
