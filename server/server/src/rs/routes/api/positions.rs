use common::ncryptflib as ncryptf;
use common::structs::packet::{
    PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::traits::player_data::PlayerData;
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
use std::collections::HashSet;
use moka::sync::Cache;
use std::time::Duration;

/// Cache of registered player names to avoid repeated database queries
#[derive(Clone)]
pub struct RegisteredPlayersCache {
    cache: Cache<String, bool>,
}

impl RegisteredPlayersCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(86400)) // 1 day
                .max_capacity(512)
                .build(),
        }
    }

    pub fn contains(&self, player_name: &str) -> bool {
        self.cache.get(player_name).is_some()
    }

    pub fn insert(&self, player_name: String) {
        self.cache.insert(player_name, true);
    }
}

const PLAYERS_PER_CHUNK: usize = 30;

/// Stores player position data
#[post("/position", data = "<positions>")]
pub async fn update_position(
    _access_token: MCAccessToken,
    db: SeaOrmConnection<'_, AppDb>,
    positions: Json<common::GameDataCollection>,
    config: &State<ApplicationConfigServer>,
    webhook_receiver: &State<WebhookReceiver>,
    registered_players: &State<RegisteredPlayersCache>,
) -> Status {
    let conn = db.into_inner();
    let (root_certificate, keypair) = match get_root_ca(config.tls.certs_path.clone()) {
        Ok((root_certificate, keypair)) => (root_certificate, keypair),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Status::Ok;
        }
    };

    // Collect all player names and filter out those we know are registered
    let all_players: Vec<_> = positions.0.players.clone();
    let player_names: Vec<String> = all_players
        .iter()
        .map(|p| p.get_name().to_string())
        .collect();

    // Filter out players already in cache
    let players_to_check: Vec<String> = player_names
        .iter()
        .filter(|name| !registered_players.contains(name))
        .cloned()
        .collect();

    // If there are players to check, do a single batch query
    if !players_to_check.is_empty() {
        match player::Entity::find()
            .filter(player::Column::Gamertag.is_in(players_to_check.clone()))
            .all(conn)
            .await
        {
            Ok(existing_players) => {
                // Collect existing player names
                let existing_names: HashSet<String> = existing_players
                    .iter()
                    .filter_map(|p| p.gamertag.clone())
                    .collect();

                // Add existing players to cache
                for name in &existing_names {
                    registered_players.insert(name.clone());
                }

                // Find players that don't exist in DB
                let new_players: Vec<String> = players_to_check
                    .into_iter()
                    .filter(|name| !existing_names.contains(name))
                    .collect();

                // Create new player records
                for player_name in new_players {
                    let kp = ncryptf::Keypair::new();
                    let signature = ncryptf::Signature::new();

                    let mut kpv = Vec::<u8>::new();
                    kpv.append(&mut kp.get_public_key());
                    kpv.append(&mut kp.get_secret_key());
                    let mut sgv = Vec::<u8>::new();
                    sgv.append(&mut signature.get_public_key());
                    sgv.append(&mut signature.get_secret_key());

                    let (cert, key) = match sign_cert_with_ca(
                        &root_certificate,
                        &keypair,
                        player_name.as_str(),
                    ) {
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
                        Ok(_) => {
                            // Add to cache after successful insert
                            registered_players.insert(player_name.clone());
                        }
                        Err(e) => {
                            tracing::error!(
                                "Unable to insert record into database. {}",
                                e.to_string()
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to query database: {}", e.to_string());
            }
        }
    }

    // Send players to webhook receiver in chunks
    let mut player_buffer = Vec::with_capacity(PLAYERS_PER_CHUNK);
    for player in all_players {
        player_buffer.push(player);

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

async fn send_player_chunk(players: &[common::PlayerEnum], webhook_receiver: &WebhookReceiver) {
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

#[get("/position")]
pub async fn position(
    // Guard the request so it's only accepted if we have a valid access token
    _access_token: MCAccessToken,
    // Cache manager state for accessing player positions
    cache_manager: &State<CacheManager>,
) -> Json<Vec<common::PlayerEnum>> {
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
