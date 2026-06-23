use common::consts::version::PROTOCOL_VERSION;
use common::response::{ApiConfigBedrock, ApiConfigResponse};
use rocket::{State, serde::json::Json};
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
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: get_config]
        },
    }
}

#[openapi(tag = "Server")]
#[get("/config")]
pub async fn get_config(config: &State<Server>, voice: &State<Voice>) -> Json<ApiConfigResponse> {
    let bedrock = {
        #[cfg(feature = "bedrock")]
        {
            let enabled = config.bedrock.enabled;
            ApiConfigBedrock {
                enabled,
                dns_enabled: config.bedrock.dns.enabled,
                transfer_port: enabled.then_some(config.bedrock.transfer_port),
            }
        }
        #[cfg(not(feature = "bedrock"))]
        {
            ApiConfigBedrock::default()
        }
    };

    Json(ApiConfigResponse {
        status: String::from("Ok"),
        client_id: config.minecraft.client_id.clone(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        quic_port: config.quic_port,
        spatial_audio: voice.spatial_audio.clone(),
        bedrock,
    })
}
