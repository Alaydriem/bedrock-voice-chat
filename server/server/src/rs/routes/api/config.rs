use common::consts::version::PROTOCOL_VERSION;
use common::structs::config::ApiConfig;
use rocket::{serde::json::Json, State};

use crate::config::{Server, Voice};

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
