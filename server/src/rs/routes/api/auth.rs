use common::{
    pool::redis::RedisDb, rocket::http::Status, rocket::serde::json::Json, rocket::State,
    rocket_db_pools::Connection as RedisConnection, structs::config::LoginRequest,
};

use crate::config::ApplicationConfigServer;
#[allow(unused_imports)] // for rust-analyzer
use common::rocket_db_pools::deadpool_redis::redis::AsyncCommands;

/// Authenticates the Player to Xbox Live to grab their gamertag and other identifying information
#[post("/", data = "<payload>")]
pub async fn authenticate(
    // Data is to be stored in Redis
    rdb: RedisConnection<RedisDb>,
    // The player OAuth2 Code
    payload: Json<LoginRequest>,
    // The application state
    config: &State<ApplicationConfigServer>,
) -> Status {
    let oauth2_transaction_code = payload.0.code;

    tracing::info!("OAuth2 Transaction Code: {}", oauth2_transaction_code);
    let client_id = config.minecraft.client_id.clone();
    let client_secret = config.minecraft.client_secret.clone();

    match common::auth::xbl::server_authenticate_with_client_code(
        client_id,
        client_secret,
        oauth2_transaction_code,
    )
    .await
    {
        Ok(params) => {
            tracing::info!("{:?}", params);
            Status::Ok
        }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Status::InternalServerError
        }
    }
}
