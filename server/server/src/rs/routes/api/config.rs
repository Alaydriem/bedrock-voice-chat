use common::consts::version::PROTOCOL_VERSION;
use common::structs::config::ApiConfig;
use rocket::{serde::json::Json, State};

use crate::config::{ApplicationConfigServer, ApplicationConfigVoice};

#[get("/config")]
pub async fn get_config(
    config: &State<ApplicationConfigServer>,
    voice: &State<ApplicationConfigVoice>,
) -> Json<ApiConfig> {
    Json(ApiConfig {
        status: String::from("Ok"),
        client_id: config.minecraft.client_id.clone(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        spatial_audio: voice.spatial_audio.clone(),
    })
}
