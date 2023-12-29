use std::{fs, path::Path};

use common::pool::redis::RedisDb;
use rocket::{
    http::Status,
    serde::json::Json
};

use rocket_db_pools::Connection as RedisConnection;
use sea_orm::ActiveValue;

use common::ncryptflib::rocket::Utc;

use entity::player;

use crate::{config::ApplicationConfigServer, rs::guards::MCAccessToken};

#[allow(unused_imports)] // for rust-analyzer
use rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use common::{
    pool::seaorm::AppDb,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use sea_orm_rocket::Connection as SeaOrmConnection;
use rcgen::{Certificate, CertificateParams, KeyPair};
use rocket::State;

use super::{create_player_certificate, is_certificate_expiring};

/// Stores player position data and online status from Minecraft Bedrock into Redis
#[post("/", data = "<positions>")]
pub async fn position(
    // Guard the request so it's only accepted if we have a valid access token
    _access_token: MCAccessToken,
    // Data is to be stored in Redis
    rdb: RedisConnection<RedisDb>,
    // Database connection
    db: SeaOrmConnection<'_, AppDb>,
    // The player position data
    positions: Json<Vec<common::Player>>,
    // Certificates path from the State Configuration
    config: &State<ApplicationConfigServer>,
) -> Status {
    let mut redis = rdb.into_inner();
    let conn = db.into_inner();

    // Generate the paths
    let certificate_path = config.tls.certs_path.clone();
    let root_ca_path_str = format!("{}/{}", &certificate_path, "ca.crt");
    let root_ca_key_path_str = format!("{}/{}", &certificate_path, "ca.key");
    let root_kp = KeyPair::from_pem(&fs::read_to_string(root_ca_key_path_str).unwrap()).unwrap();
    let root_cp = CertificateParams::from_ca_cert_pem(
        &fs::read_to_string(root_ca_path_str).unwrap(),
        root_kp,
    )
    .unwrap();
    let root_certificate = Certificate::from_params(root_cp).unwrap();

    // Iterate through each of the players
    for player in positions.0 {
        let player_name = player.clone().name;
        let player_certificate_path = format!("{}/{}.crt", &certificate_path, &player_name);

        let is_banished: bool = match redis
            .exists(common::pool::redis::create_redis_key(
                common::consts::redis::REDIS_KEY_BANISHED_QUERY,
                &player_name,
            ))
            .await
        {
            Ok(exists) => match exists {
                true => match redis
                    .get(common::pool::redis::create_redis_key(
                        common::consts::redis::REDIS_KEY_BANISHED_QUERY,
                        &player_name,
                    ))
                    .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::error!("Unable to connect to Redis: {}", e.to_string());
                        true
                    }
                },
                false => {
                    match player::Entity::find()
                        .filter(player::Column::Gamertag.eq(player_name.clone()))
                        .one(conn)
                        .await
                    {
                        Ok(record) => {
                            match record {
                                Some(r) => {
                                    // Store the value in redis for a day
                                    let (): () = match redis
                                        .set_ex(
                                            common::pool::redis::create_redis_key(
                                                common::consts::redis::REDIS_KEY_BANISHED_QUERY,
                                                &player_name,
                                            ),
                                            r.banished,
                                            3600 as usize,
                                        )
                                        .await
                                    {
                                        Ok(r) => r,
                                        Err(e) => {
                                            tracing::error!("Unable to update cached value for if player is banished: {}", e.to_string());
                                        }
                                    };

                                    r.banished
                                }
                                None => {
                                    // We didn't get a record, so create one
                                    let p = player::ActiveModel {
                                        id: ActiveValue::NotSet,
                                        gamertag: ActiveValue::Set(Some(player_name.clone())),
                                        gamerpic: ActiveValue::Set(None),
                                        banished: ActiveValue::Set(false),
                                        created_at: ActiveValue::Set(Utc::now().timestamp() as u32),
                                        updated_at: ActiveValue::Set(Utc::now().timestamp() as u32),
                                    };

                                    // Insert the record
                                    match p.insert(conn).await {
                                        Ok(_) => false,
                                        Err(e) => {
                                            tracing::error!(
                                                "Unable to insert record into database. {}",
                                                e.to_string()
                                            );
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to connect to database: {}", e.to_string());

                            // There's nothing we can do for this player if this happens, continue
                            continue;
                        }
                    }
                }
            },
            Err(e) => {
                tracing::error!("Unable to connect to Redis: {}", e.to_string());
                true
            }
        };

        // Don't rekey if the user is banished
        if is_banished {
            tracing::info!("Player was banished, not continuing");
            continue;
        }

        // If the certificate for the player doesn't exist, create it.
        if !Path::new(&player_certificate_path).exists()
            || is_certificate_expiring(&certificate_path, &player_name)
        {
            match create_player_certificate(
                &player_name,
                certificate_path.clone(),
                &root_certificate,
            ) {
                Ok(_) => {
                    tracing::info!("Certificate and keypair created for {}", &player_name);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create certificate or keypair for {}. {}",
                        &player_name,
                        e.to_string()
                    );
                }
            };
        }

        // Store the player position data in Redis
        // annoying KV/RV set value is annoying
        let mut _result = match redis
            .set(
                common::pool::redis::create_redis_key(
                    common::consts::redis::REDIS_KEY_PLAYER_POSITION,
                    &player_name,
                ),
                serde_json::to_string(&player).unwrap(),
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Unable to write to Redis. {}", e.to_string());
            }
        };
    }
    return Status::Ok;
}
