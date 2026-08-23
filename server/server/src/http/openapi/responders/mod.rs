use okapi::openapi3::Responses;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;

mod ncryptf;
mod required;

pub use ncryptf::NcryptfJsonResponse;
pub use required::CustomJsonResponseRequired;

/// Wrapper for `status::Custom<Option<Json<T>>>` that implements `OpenApiResponderInner`.
pub struct CustomJsonResponse<T> {
    pub status: Status,
    pub body: Option<T>,
}

impl<T> CustomJsonResponse<T> {
    pub fn ok(body: T) -> Self {
        Self {
            status: Status::Ok,
            body: Some(body),
        }
    }

    pub fn error(status: Status) -> Self {
        Self { status, body: None }
    }

    pub fn custom(status: Status, body: Option<T>) -> Self {
        Self { status, body }
    }
}

impl<'r, T: serde::Serialize + Send + 'static> Responder<'r, 'static> for CustomJsonResponse<T> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        match self.body {
            Some(body) => {
                rocket::response::status::Custom(self.status, Json(body)).respond_to(request)
            }
            None => rocket::Response::build().status(self.status).ok(),
        }
    }
}

impl<T: serde::Serialize + schemars::JsonSchema + Send + 'static> OpenApiResponderInner
    for CustomJsonResponse<T>
{
    fn responses(generator: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        <Json<T>>::responses(generator)
    }
}

