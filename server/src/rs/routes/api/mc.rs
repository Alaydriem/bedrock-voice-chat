use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use common::{
    pool::redis::RedisDb, rocket::http::Status, rocket::serde::json::Json,
    rocket_db_pools::Connection as RedisConnection, sea_orm::ActiveValue,
};

use common::{ncryptflib::rocket::Utc, sea_orm_migration::DbErr};

use anyhow::anyhow;
use entity::player;

use crate::{config::ApplicationConfigServer, rs::guards::MCAccessToken};

#[allow(unused_imports)] // for rust-analyzer
use common::rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use common::{
    pool::seaorm::AppDb,
    sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set},
    sea_orm_rocket::Connection as SeaOrmConnection,
};
use rcgen::{Certificate, CertificateParams, KeyPair, PKCS_ED25519};
use rocket::{
    time::{Duration, OffsetDateTime},
    State,
};

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
                                            tracing::error!("Unable to update cached value for if player is banished");
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
                            tracing::error!("Failed to connect to database");

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

fn is_certificate_expiring(certificate_path: &str, player_name: &str) -> bool {
    let player_cert_path_str = format!("{}/{}.crt", &certificate_path, player_name);
    let player_key_path_str = format!("{}/{}.key", &certificate_path, player_name);

    let kp = KeyPair::from_pem(&fs::read_to_string(player_key_path_str).unwrap()).unwrap();
    let cp =
        CertificateParams::from_ca_cert_pem(&fs::read_to_string(player_cert_path_str).unwrap(), kp)
            .unwrap();

    // If the certificate is expiring in 15 days, renew it.
    if cp.not_after
        <= OffsetDateTime::now_utc()
            .checked_sub(Duration::days(-15))
            .unwrap()
    {
        return true;
    }

    return false;
}

/// Creates a new certificate and keypair for the given player from their name to the certificates_path directory
/// This certificate is signed by the root CA for both mTLS and QUIC MoQ Transport
fn create_player_certificate(
    player_name: &str,
    certificate_path: String,
    root_certificate: &Certificate,
) -> Result<Certificate, anyhow::Error> {
    let player_kp = match KeyPair::generate(&PKCS_ED25519) {
        Ok(r) => r,
        Err(_) => return Err(anyhow!("Unable to generate keypair")),
    };

    let mut distinguished_name = rcgen::DistinguishedName::new();
    distinguished_name.push(rcgen::DnType::CommonName, player_name);

    let mut cp = CertificateParams::new(vec![player_name.to_string().clone()]);
    cp.alg = &PKCS_ED25519;
    cp.not_before = OffsetDateTime::now_utc();
    // Certificates are valid for 90 days
    // Having the server advertise the user being connected to the server will automatically issue a new certificate
    // The client will need to retrieve the updated certificate to continue working 15 days or less prior
    // If the client certificate is invalid we bounce them
    // Certificates aren't revoked, the only expire
    cp.not_after = cp.not_before.checked_add(Duration::days(90)).unwrap();
    cp.distinguished_name = distinguished_name;
    cp.key_pair = Some(player_kp);

    let player_certificate = match Certificate::from_params(cp) {
        Ok(c) => c,
        Err(_) => return Err(anyhow!("Unable to generate certificate")),
    };

    // This is the signed player certificate
    let signed_player_certificate = player_certificate
        .serialize_pem_with_signer(&root_certificate)
        .unwrap();

    let key: String = player_certificate.get_key_pair().serialize_pem();

    let player_cert_path_str = format!("{}/{}.crt", &certificate_path, player_name);
    let player_key_path_str = format!("{}/{}.key", &certificate_path, player_name);

    let mut key_file = File::create(player_cert_path_str).unwrap();
    key_file
        .write_all(signed_player_certificate.as_bytes())
        .unwrap();
    let mut cert_file = File::create(player_key_path_str).unwrap();
    cert_file.write_all(key.as_bytes()).unwrap();

    return Ok(player_certificate);
}
