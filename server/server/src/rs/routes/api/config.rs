use common::structs::config::ApiConfig;
use rocket::{serde::json::Json, State};

use crate::config::ApplicationConfigServer;
#[get("/config")]
pub async fn get_config(config: &State<ApplicationConfigServer>) -> Json<ApiConfig> {
    Json(ApiConfig {
        status: String::from("Ok"),
        client_id: config.minecraft.client_id.clone(),
    })
}
