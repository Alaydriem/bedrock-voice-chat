use common::ncryptflib::rocket::JsonResponse as NcryptfRocketResponse;
use okapi::openapi3::Responses;
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;

use crate::http::dtos::ncryptf::JsonMessage;

/// Wrapper around `ncryptf::rocket::JsonResponse<JsonMessage<T>>` that documents the plaintext
/// response schema in the OpenAPI spec. The actual wire format may be ncryptf-encrypted
/// depending on the client's `Accept` header.
pub struct NcryptfJsonResponse<T: serde::Serialize>(pub NcryptfRocketResponse<JsonMessage<T>>);

impl<T: serde::Serialize> NcryptfJsonResponse<T> {
    pub fn from_inner(inner: NcryptfRocketResponse<JsonMessage<T>>) -> Self {
        Self(inner)
    }
}

impl<'r, T: serde::Serialize + Send + 'static> Responder<'r, 'static> for NcryptfJsonResponse<T> {
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
