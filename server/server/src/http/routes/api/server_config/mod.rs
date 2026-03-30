use common::consts::version::PROTOCOL_VERSION;
use common::response::ApiConfig;
use rocket::{serde::json::Json, State};
use rocket_okapi::openapi;

use crate::config::{Server, Voice};
use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "Server",
        description: "Server configuration and metadata. Returns connection details, \
                      protocol version, and spatial audio settings.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: get_config]
        },
    }
}

#[openapi(tag = "Server")]
#[get("/config")]
pub async fn get_config(
    config: &State<Server>,
    voice: &State<Voice>,
) -> Json<ApiConfig> {
    Json(ApiConfig {
        status: String::from("Ok"),
        client_id: config.minecraft.client_id.clone(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        quic_port: config.quic_port,
        spatial_audio: voice.spatial_audio.clone(),
    })
}
