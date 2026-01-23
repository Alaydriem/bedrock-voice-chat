//! ncryptf JSON message wrapper for API responses

use common::ncryptflib::rocket::{Json, JsonResponse};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

use super::JsonError;

/// Standard JSON message wrapper for API responses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonMessage<T: serde::Serialize> {
    pub status: u16,
    pub data: Option<T>,
    pub message: Option<String>,
    pub errors: Option<JsonError>,
}

impl<T: serde::Serialize> JsonMessage<T> {
    /// Creates a new JsonResponse from a given struct or errors
    pub fn create(
        status: Status,
        result: Option<T>,
        errors: Option<JsonError>,
        message: Option<&str>,
    ) -> JsonResponse<JsonMessage<T>> {
        JsonResponse {
            status,
            json: Json(JsonMessage {
                status: status.code,
                data: result,
                message: message.map(|m| m.to_string()),
                errors,
            }),
        }
    }
}
