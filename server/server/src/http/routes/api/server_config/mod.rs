use common::consts::version::PROTOCOL_VERSION;
use common::response::{
    ApiConfigAge, ApiConfigBedrock, ApiConfigCapacity, ApiConfigChat, ApiConfigRecording,
    ApiConfigResponse,
};
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

/// Return the configuration a client needs before connecting.
///
/// Unauthenticated, and the usual way to confirm a server is reachable and its TLS is
/// valid.
#[openapi(tag = "Server")]
#[get("/config")]
pub async fn get_config(
    config: &State<Server>,
    voice: &State<Voice>,
    cache_manager: &State<crate::stream::quic::CacheManager>,
) -> Json<ApiConfigResponse> {
    let in_use = cache_manager
        .get_connection_registry()
        .map(|registry| registry.on_voice_identities().len() as u32)
        .unwrap_or(0);

    let bedrock = {
        #[cfg(feature = "bedrock")]
        {
            config.bedrock.to_api()
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
        quic_ports: config.quic_ports(),
        // Unconditional in this build: `ServerRuntime` binds the WebSocket voice listener
        // alongside QUIC and hands the demultiplexer its address, with nothing for an
        // operator to turn on. Should that ever become optional, this has to follow it —
        // a client reads this field to decide whether a fallback path is worth probing.
        voice_websocket: true,
        spatial_audio: voice.spatial_audio.clone(),
        bedrock,
        age: ApiConfigAge::from_minimum(config.age.minimum),
        recording: ApiConfigRecording {
            enabled: voice.recording.enabled,
        },
        chat: ApiConfigChat {
            enabled: config.features.chat,
        },
        capacity: ApiConfigCapacity {
            limit: voice.limits.connections,
            in_use,
        },
    })
}
