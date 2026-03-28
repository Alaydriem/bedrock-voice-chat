use common::ncryptflib::rocket::JsonResponse as NcryptfRocketResponse;
use okapi::openapi3::Responses;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;

use crate::http::dtos::ncryptf::JsonMessage;

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
        rocket::response::status::Custom(self.status, self.body.map(Json)).respond_to(request)
    }
}

impl<T: serde::Serialize + schemars::JsonSchema + Send + 'static> OpenApiResponderInner
    for CustomJsonResponse<T>
{
    fn responses(generator: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        <Json<T>>::responses(generator)
    }
}

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

/// Wrapper around `ncryptf::rocket::JsonResponse<JsonMessage<T>>` that documents the plaintext
/// response schema in the OpenAPI spec. The actual wire format may be ncryptf-encrypted
/// depending on the client's `Accept` header.
pub struct NcryptfJsonResponse<T: serde::Serialize>(pub NcryptfRocketResponse<JsonMessage<T>>);

impl<T: serde::Serialize> NcryptfJsonResponse<T> {
    pub fn from_inner(inner: NcryptfRocketResponse<JsonMessage<T>>) -> Self {
        Self(inner)
    }
}

impl<'r, T: serde::Serialize + Send + 'static> Responder<'r, 'static>
    for NcryptfJsonResponse<T>
{
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        self.0.respond_to(request)
    }
}

impl<T: serde::Serialize + schemars::JsonSchema + Send + 'static> OpenApiResponderInner
    for NcryptfJsonResponse<T>
{
    fn responses(generator: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        <Json<JsonMessage<T>>>::responses(generator)
    }
}
