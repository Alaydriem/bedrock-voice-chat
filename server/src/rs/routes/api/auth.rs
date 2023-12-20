use anyhow::anyhow;
use common::{
    auth::xbl::ProfileResponse,
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

    let profile = match common::auth::xbl::server_authenticate_with_client_code(
        client_id,
        client_secret,
        oauth2_transaction_code,
    )
    .await
    {
        // We should only ever get a single user back, if we get none, or more than one then...
        // something not right
        Ok(params) => match params.profile_users.len() {
            0 => None,
            1 => Some(params),
            _ => None,
        },
        Err(e) => {
            tracing::error!("{}", e.to_string());
            None
        }
    };

    let (gamerpic, gamertag) = match get_gamertag_and_gamepic(profile) {
        Ok((gamerpic, gamertag)) => (gamerpic, gamertag),
        Err(e) => {
            return JsonMessage::create(Status::Forbidden, None, None, Some(e.to_string().as_str()))
        }
    };

    let response = LoginResponse {
        key: String::from("foo"),
        cert: String::from("foo2"),
        gamerpic: gamerpic,
        gamertag: gamertag,
    };
    return JsonMessage::create(Status::Ok, Some(response), None, None);

    // Unauthorized by default as a fallthrough
    return JsonMessage::create(
        Status::Unauthorized,
        None,
        None,
        Some("Unable to login to Microsoft Services"),
    );
}

/// Extracts the gamerpicture and gamertag from the profile response
fn get_gamertag_and_gamepic(
    profile: Option<ProfileResponse>,
) -> Result<(String, String), anyhow::Error> {
    match profile {
        Some(profile) => {
            let mut gamerpic: Option<String> = None;
            let mut gamertag: Option<String> = None;

            for setting in profile.profile_users[0].settings.clone().into_iter() {
                if setting.id.eq("GamerDisplayPicRaw") {
                    gamerpic = Some(setting.value.clone());
                }

                if setting.id.eq("Gamertag") {
                    gamertag = Some(setting.value.clone());
                }
            }

            if gamerpic.is_some() && gamertag.is_some() {
                return Ok((gamerpic.unwrap(), gamertag.unwrap()));
            }

            return Err(anyhow!("Authentication was successful, but the profile did not include the expected attributes."));
        }
        None => {
            return Err(anyhow!(
                "Authentication to Microsoft services was unsucccessful."
            ))
        }
    }
}
