use common::{
    pool::redis::RedisDb,
    rocket::http::Status,
    rocket::State,
    rocket::serde::json::Json,
    rocket_db_pools::Connection as RedisConnection,
};

#[allow(unused_imports)] // for rust-analyzer
use common::rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use crate::config::ApplicationConfigServer;

/// Authenticates the Player to Xbox Live to grab their gamertag and other identifying information
#[post("/", data = "<code>")]
pub async fn authenticate(
    // Data is to be stored in Redis
    rdb: RedisConnection<RedisDb>,
    // The player OAuth2 Code
    code: Json<String>,
    // The application state
    config: &State<ApplicationConfigServer>
) -> Status {
    let oauth2_transaction_code = code.0;
    let client_id = config.minecraft.client_id.clone();
    let client_secret = config.minecraft.client_secret.clone();

    match common::auth::xbl::server_authenticate_with_client_code(client_id, client_secret, oauth2_transaction_code).await {
        Ok(params) => {
            tracing::info!("{:?}", params);
            Status::Ok
        },
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Status::InternalServerError
        }
    }
}