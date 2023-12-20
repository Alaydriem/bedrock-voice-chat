use common::rocket::{serde::json::Json, State};
use common::structs::config::ApiConfig;

use crate::config::ApplicationConfigServer;
#[get("/")]
pub async fn get_config(config: &State<ApplicationConfigServer>) -> Json<ApiConfig> {
    Json(ApiConfig {
        status: String::from("Ok"),
        client_id: config.minecraft.client_id.clone(),
    })
}
