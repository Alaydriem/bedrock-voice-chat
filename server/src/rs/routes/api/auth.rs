use common::{
    ncryptflib as ncryptf,
    pool::redis::RedisDb,
    rocket::http::Status,
    rocket::serde::json::Json,
    rocket::State,
    rocket_db_pools::Connection as RedisConnection,
    structs::{
        config::{LoginRequest, LoginResponse},
        ncryptf_json::JsonMessage,
    },
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
) -> ncryptf::rocket::JsonResponse<JsonMessage<LoginResponse>> {
    let oauth2_transaction_code = payload.0.code;

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
            let mut gamerpic: String = String::from("gamerpic");
            let mut gamertag: String = String::from("gamertag");

            // We should have at least one user, if we don't then we didn't get a valid response back
            if params.profile_users.len() == 0 {
                return JsonMessage::create(
                    Status::Forbidden,
                    None,
                    None,
                    Some("Unable to login to Microsoft Services"),
                );
            }

            for setting in params.profile_users[0].settings.clone().into_iter() {
                if setting.id.eq("GamerDisplayPicRaw") {
                    gamerpic = setting.value.clone();
                }

                if setting.id.eq("Gamertag") {
                    gamertag = setting.value.clone();
                }
            }

            let response = LoginResponse {
                key: String::from("foo"),
                cert: String::from("foo2"),
                gamerpic,
                gamertag,
            };
            return JsonMessage::create(Status::Ok, Some(response), None, None);
        }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return JsonMessage::create(
                Status::Forbidden,
                None,
                None,
                Some("Unable to login to Microsoft Services"),
            );
        }
    }
}
