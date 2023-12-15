use common::rocket::serde::json::Json;
use common::structs::config::ApiConfig;

#[get("/")]
pub async fn get_config() -> Json<ApiConfig> {
    Json(ApiConfig {
        status: String::from("Ok"),
    })
}
