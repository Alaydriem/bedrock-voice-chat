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

impl<T: serde::Serialize + schemars::JsonSchema> schemars::JsonSchema for JsonMessage<T> {
    fn schema_name() -> String {
        format!("JsonMessage_{}", T::schema_name())
    }

    fn json_schema(r#gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::*;

        let obj = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            object: Some(Box::new(ObjectValidation {
                properties: {
                    let mut props = schemars::Map::new();
                    props.insert("status".into(), r#gen.subschema_for::<u16>());
                    props.insert("data".into(), r#gen.subschema_for::<Option<T>>());
                    props.insert("message".into(), r#gen.subschema_for::<Option<String>>());
                    props.insert("errors".into(), r#gen.subschema_for::<Option<JsonError>>());
                    props
                },
                required: {
                    let mut req = std::collections::BTreeSet::new();
                    req.insert("status".into());
                    req
                },
                ..Default::default()
            })),
            ..Default::default()
        };
        Schema::Object(obj)
    }
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
