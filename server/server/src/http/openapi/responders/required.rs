use okapi::openapi3::Responses;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;

/// Wrapper for `status::Custom<Json<T>>` that implements `OpenApiResponderInner`.
pub struct CustomJsonResponseRequired<T> {
    pub status: Status,
    pub body: T,
}

impl<T> CustomJsonResponseRequired<T> {
    pub fn ok(body: T) -> Self {
        Self {
            status: Status::Ok,
            body,
        }
    }

    pub fn custom(status: Status, body: T) -> Self {
        Self { status, body }
    }
}

impl<'r, T: serde::Serialize + Send + 'static> Responder<'r, 'static>
    for CustomJsonResponseRequired<T>
{
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        rocket::response::status::Custom(self.status, Json(self.body)).respond_to(request)
    }
}

impl<T: serde::Serialize + schemars::JsonSchema + Send + 'static> OpenApiResponderInner
    for CustomJsonResponseRequired<T>
{
    fn responses(generator: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        <Json<T>>::responses(generator)
    }
}
