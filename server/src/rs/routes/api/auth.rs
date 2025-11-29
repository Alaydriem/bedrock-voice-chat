use std::path::Path;
use base64::{Engine as _, engine::general_purpose};
use anyhow::anyhow;
use common::{
    auth::xbl::ProfileResponse,
    ncryptflib as ncryptf,
    pool::seaorm::AppDb,
    structs::{
        config::{LoginRequest, LoginResponse},
        ncryptf_json::JsonMessage,
    },
};
use rocket::{http::Status, serde::json::Json, State};

use entity::player;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm_rocket::Connection as SeaOrmConnection;

use crate::config::ApplicationConfigServer;

/// Authenticates the Player to Xbox Live to grab their gamertag and other identifying information
#[post("/auth", data = "<payload>")]
pub async fn authenticate(
    // Database connection
    db: SeaOrmConnection<'_, AppDb>,
    // The player OAuth2 Code
    payload: Json<LoginRequest>,
    // The application state
    config: &State<ApplicationConfigServer>,
) -> ncryptf::rocket::JsonResponse<JsonMessage<LoginResponse>> {
    let conn = db.into_inner();

    let oauth2_transaction_code = payload.0.code;

    let client_id = config.minecraft.client_id.clone();
    let client_secret = config.minecraft.client_secret.clone();

    let profile = match common::auth::xbl::server_authenticate_with_client_code(
        client_id,
        client_secret,
        oauth2_transaction_code,
        payload.0.redirect_uri.clone().parse().unwrap(),
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
        Err(_e) => None,
    };

    let (gamerpic, gamertag) = match get_gamertag_and_gamepic(profile) {
        Ok((gamerpic, gamertag)) => (gamerpic, gamertag),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return JsonMessage::create(Status::Forbidden, None, None, None);
        }
    };

    let p = match player::Entity::find()
        .filter(player::Column::Gamertag.eq(gamertag.clone()))
        .one(conn)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            None
        }
    };

    let actual = match p {
        Some(p) => p,
        None => {
            tracing::info!("{:?} didn't match", p);
            return JsonMessage::create(Status::Forbidden, None, None, None);
        }
    };

    // Block banished users from logging in
    match actual.banished {
        true => {
            tracing::info!("Banished");
            return JsonMessage::create(Status::Forbidden, None, None, None);
        }
        false => {}
    }

    let kp = actual.get_keypair().unwrap();
    let sp = actual.get_signature().unwrap();

    let response = LoginResponse {
        gamertag,
        gamerpic,
        keypair: common::structs::config::Keypair {
            pk: kp.get_public_key(),
            sk: kp.get_public_key(),
        },
        signature: common::structs::config::Keypair {
            pk: sp.get_public_key(),
            sk: sp.get_public_key(),
        },
        certificate: actual.certificate,
        certificate_key: actual.certificate_key,
        certificate_ca: std::fs::read_to_string(Path::new(&format!(
            "{}/ca.crt",
            config.tls.certs_path
        )))
        .unwrap(),
        quic_connect_string: config.quic_port.to_string(),
    };

    return JsonMessage::create(Status::Ok, Some(response), None, None);
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
                if setting.id == "GameDisplayPicRaw" {
                    gamerpic = Some(general_purpose::STANDARD.encode(setting.value.clone()));
                }

                if setting.id == "Gamertag" {
                    gamertag = Some(setting.value.clone());
                }
            }

            if gamerpic.is_some() && gamertag.is_some() {
                return Ok((gamerpic.unwrap(), gamertag.unwrap()));
            }

            return Err(
                anyhow!(
                    "Authentication was successful, but the profile did not include the expected attributes."
                )
            );
        }
        None => {
            return Err(anyhow!(
                "Authentication to Microsoft services was unsucccessful."
            ));
        }
    }
}
