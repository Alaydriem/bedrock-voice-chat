use common::ncryptflib as ncryptf;
use common::structs::packet::{
    PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use rocket::{http::Status, serde::json::Json, State};

use sea_orm::ActiveValue;

use crate::rs::certificates::{get_root_ca, sign_cert_with_ca};
use common::ncryptflib::rocket::Utc;
use entity::player;

use crate::{
    config::ApplicationConfigServer,
    rs::guards::MCAccessToken,
    rs::pool::AppDb,
    stream::quic::{CacheManager, WebhookReceiver},
};
#[allow(unused_imports)] // for rust-analyzer
use rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use sea_orm_rocket::Connection as SeaOrmConnection;

const PLAYERS_PER_CHUNK: usize = 30;

/// Stores player position data and online status from Minecraft Bedrock into Redis
#[post("/mc", data = "<positions>")]
pub async fn update_position(
    _access_token: MCAccessToken,
    db: SeaOrmConnection<'_, AppDb>,
    positions: Json<common::GameData>,
    config: &State<ApplicationConfigServer>,
    webhook_receiver: &State<WebhookReceiver>,
) -> Status {
    let conn = db.into_inner();
    let (root_certificate, keypair) = match get_root_ca(config.tls.certs_path.clone()) {
        Ok((root_certificate, keypair)) => (root_certificate, keypair),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Status::Ok;
        }
    };

    let mut player_buffer = Vec::with_capacity(PLAYERS_PER_CHUNK);

    // Single loop through all players
    for player in positions.0.players {
        let player_name = &player.name;

        // Database operations
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
                continue;
            }
        }

        // Add player to buffer
        player_buffer.push(player);

        // Send chunk when buffer is full
        if player_buffer.len() >= PLAYERS_PER_CHUNK {
            send_player_chunk(&player_buffer, webhook_receiver).await;
            player_buffer.clear();
        }
    }

    // Send any remaining players
    if !player_buffer.is_empty() {
        send_player_chunk(&player_buffer, webhook_receiver).await;
    }

    Status::Ok
}

async fn send_player_chunk(players: &[common::Player], webhook_receiver: &WebhookReceiver) {
    let packet = QuicNetworkPacket {
        owner: Some(PacketOwner {
            name: String::from("api"),
            client_id: vec![0u8; 0],
        }),
        packet_type: PacketType::PlayerData,
        data: QuicNetworkPacketData::PlayerData(PlayerDataPacket {
            players: players.to_vec(),
        }),
    };

    if let Err(e) = webhook_receiver.send_packet(packet).await {
        tracing::error!("Failed to send packet chunk to QUIC server: {}", e);
    }
}

#[get("/mc")]
pub async fn position(
    // Guard the request so it's only accepted if we have a valid access token
    _access_token: MCAccessToken,
    // Cache manager state for accessing player positions
    cache_manager: &State<CacheManager>,
) -> Json<Vec<common::Player>> {
    // Get all current player positions from the cache
    let player_cache = cache_manager.get_player_cache();

    // Collect all cached players
    let mut players = Vec::new();

    for (_, player) in player_cache.iter() {
        players.push(player.clone());
    }

    // Return the collected players
    Json(players)
}
