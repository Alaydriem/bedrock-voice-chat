use common::ncryptflib::rocket::{Json, JsonResponse};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonError {
    /// Any JSON serializable struct
    pub errors: HashMap<String, String>,
}

impl JsonError {
    /// Creates a new JsonError representation
    pub fn new() -> Self {
        return Self {
            errors: HashMap::<String, String>::new(),
        };
    }

    /// Utility method to add an error
    pub fn add_error(&self, key: &str, value: &str) -> Self {
        let mut errors = self.errors.clone();
        errors.insert(key.to_string(), value.to_string());
        return Self { errors: errors };
    }
}

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
        return JsonResponse {
            status,
            json: Json(JsonMessage {
                status: status.code,
                data: result,
                message: match message {
                    Some(message) => Some(message.to_string()),
                    None => None,
                },
                errors,
            }),
        };
    }

    pub fn to_error_status(self) -> JsonResponse<JsonMessage<T>> {
        return JsonResponse {
            status: Status::InternalServerError,
            json: Json(self),
        };
    }

    /// Returns the message if one exists
    pub fn get_message(&self) -> String {
        return match self.message.clone() {
            Some(message) => message,
            None => String::from(""),
        };
    }
}
