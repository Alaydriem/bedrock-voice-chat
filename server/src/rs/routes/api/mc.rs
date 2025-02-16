use common::structs::packet::{
    PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{ncryptflib as ncryptf, pool::redis::RedisDb};
use rocket::{http::Status, serde::json::Json, State};

use std::sync::Arc;

use rocket_db_pools::Connection as RedisConnection;
use sea_orm::ActiveValue;

use common::certificates::{get_root_ca, sign_cert_with_ca};
use common::ncryptflib::rocket::Utc;
use entity::player;

use crate::{config::ApplicationConfigServer, rs::guards::MCAccessToken};

use common::pool::seaorm::AppDb;
#[allow(unused_imports)] // for rust-analyzer
use rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use sea_orm_rocket::Connection as SeaOrmConnection;

/// Stores player position data and online status from Minecraft Bedrock into Redis
#[post("/mc", data = "<positions>")]
pub async fn position(
    // Guard the request so it's only accepted if we have a valid access token
    _access_token: MCAccessToken,
    // Data is to be stored in Redis
    _rdb: RedisConnection<RedisDb>,
    // Database connection
    db: SeaOrmConnection<'_, AppDb>,
    // The player position data
    positions: Json<Vec<common::Player>>,
    // Configuration
    config: &State<ApplicationConfigServer>,
    // Deadqueue, which is how we communicate with the QUIC server without creating a new stream
    queue: &State<Arc<deadqueue::limited::Queue<QuicNetworkPacket>>>,
) -> Status {
    let conn = db.into_inner();
    let (root_certificate, keypair) = match get_root_ca(config.tls.certs_path.clone()) {
        Ok((root_certificate, keypair)) => (root_certificate, keypair),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Status::Ok;
        }
    };

    // Iterate through each of the players
    for player in positions.0.clone() {
        let player_name = &player.name;
        match player::Entity::find()
            .filter(player::Column::Gamertag.eq(player_name))
            .one(conn)
            .await
        {
            Ok(record) => match record {
                Some(_) => {}
                None => {
                    let kp = ncryptf::Keypair::new();
                    let signature = ncryptf::Signature::new();

                    let mut kpv = Vec::<u8>::new();
                    kpv.append(&mut kp.get_public_key());
                    kpv.append(&mut kp.get_secret_key());
                    let mut sgv = Vec::<u8>::new();
                    sgv.append(&mut signature.get_public_key());
                    sgv.append(&mut signature.get_secret_key());

                    let (cert, key) =
                        match sign_cert_with_ca(&root_certificate, &keypair, &player_name) {
                            Ok((cert, key)) => (cert, key),
                            Err(e) => {
                                tracing::error!("{}", e.to_string());
                                continue;
                            }
                        };

                    // We didn't get a record, so create one
                    let p = player::ActiveModel {
                        id: ActiveValue::NotSet,
                        gamertag: ActiveValue::Set(Some(player_name.clone())),
                        gamerpic: ActiveValue::Set(None),
                        certificate: ActiveValue::Set(cert.pem()),
                        certificate_key: ActiveValue::Set(key.serialize_pem()),
                        banished: ActiveValue::Set(false),
                        keypair: ActiveValue::Set(kpv),
                        signature: ActiveValue::Set(sgv),
                        created_at: ActiveValue::Set(Utc::now().timestamp() as u32),
                        updated_at: ActiveValue::Set(Utc::now().timestamp() as u32),
                    };

                    // Insert the record
                    match p.insert(conn).await {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!(
                                "Unable to insert record into database. {}",
                                e.to_string()
                            );
                            continue;
                        }
                    }
                }
            },
            Err(e) => {
                tracing::error!("Failed to connect to database: {}", e.to_string());

                // There's nothing we can do for this player if this happens, continue
                continue;
            }
        }
    }

    // Broadcast the player position to QUIC
    let packet = QuicNetworkPacket {
        owner: PacketOwner {
            name: String::from("api"),
            client_id: vec![0u8; 0],
        },
        packet_type: PacketType::PlayerData,
        data: QuicNetworkPacketData::PlayerData(PlayerDataPacket {
            players: positions.0,
        }),
    };

    _ = queue.push(packet).await;
    return Status::Ok;
}
