//! Player registration service

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose, Engine as _};
use common::ncryptflib as ncryptf;
use common::ncryptflib::rocket::Utc;
use common::traits::player_data::PlayerData;
use common::Game;
use entity::{player, player_identity};
use moka::sync::Cache;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::services::CertificateService;

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

impl Default for RegisteredPlayersCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for player registration logic.
/// Creates new player records in the database with certificates.
/// Shared between HTTP routes and FFI to ensure players are registered
/// regardless of how position updates are received.
#[derive(Clone)]
pub struct PlayerRegistrarService {
    db: Arc<DatabaseConnection>,
    cert_service: Arc<CertificateService>,
    cache: RegisteredPlayersCache,
}

impl PlayerRegistrarService {
    /// Create a new PlayerRegistrarService.
    ///
    /// # Arguments
    /// * `db` - Database connection wrapped in Arc for sharing
    /// * `cert_service` - Certificate service for signing player certificates
    pub fn new(db: Arc<DatabaseConnection>, cert_service: Arc<CertificateService>) -> Self {
        Self {
            db,
            cert_service,
            cache: RegisteredPlayersCache::new(),
        }
    }

    /// Get a reference to the registered players cache.
    /// This allows sharing the cache with HTTP routes that use sea_orm_rocket.
    pub fn cache(&self) -> &RegisteredPlayersCache {
        &self.cache
    }

    /// Process a list of players, checking the cache and database, and creating
    /// new player records for any unregistered players.
    ///
    /// # Arguments
    /// * `players` - List of player position data
    /// * `game_type` - The game type (Minecraft, Hytale, etc.)
    pub async fn process_players(&self, players: &[common::PlayerEnum], game_type: Game) {
        // Build name → UUID map for players that have a platform UUID
        let uuid_map: HashMap<String, String> = players
            .iter()
            .filter_map(|p| {
                p.get_player_uuid()
                    .map(|uuid| (p.get_name().to_string(), uuid.to_string()))
            })
            .collect();

        // Collect all player names and filter out those we know are registered
        let player_names: Vec<String> = players.iter().map(|p| p.get_name().to_string()).collect();

        // Filter out players already in cache
        let players_to_check: Vec<String> = player_names
            .iter()
            .filter(|name| !self.cache.contains(name))
            .cloned()
            .collect();

        if players_to_check.is_empty() {
            return;
        }

        // Batch query the database for existing players
        match player::Entity::find()
            .filter(player::Column::Gamertag.is_in(players_to_check.clone()))
            .filter(player::Column::Game.eq(game_type.clone()))
            .all(self.db.as_ref())
            .await
        {
            Ok(existing_players) => {
                // Collect existing player names
                let existing_names: HashSet<String> = existing_players
                    .iter()
                    .filter_map(|p| p.gamertag.clone())
                    .collect();

                // Add existing players to cache + store UUID identity + generate gamerpic
                for existing in &existing_players {
                    if let Some(ref name) = existing.gamertag {
                        self.cache.insert(name.clone());

                        if let Some(uuid) = uuid_map.get(name) {
                            self.store_platform_uuid(existing.id, uuid, &game_type)
                                .await;

                            if game_type == Game::Hytale && existing.gamerpic.is_none() {
                                self.generate_hytale_gamerpic(existing.id, uuid).await;
                            }
                        }
                    }
                }

                // Find players that don't exist in DB
                let new_players: Vec<String> = players_to_check
                    .into_iter()
                    .filter(|name| !existing_names.contains(name))
                    .collect();

                // Create new player records
                for player_name in new_players {
                    let uuid = uuid_map.get(&player_name).map(|s| s.as_str());
                    self.create_player(&player_name, &game_type, uuid).await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to query database: {}", e.to_string());
            }
        }
    }

    /// Create a new player record in the database.
    /// If a platform UUID is provided and the game is Hytale, a gamerpic is generated.
    pub async fn create_player(
        &self,
        player_name: &str,
        game_type: &Game,
        player_uuid: Option<&str>,
    ) {
        let kp = ncryptf::Keypair::new();
        let signature = ncryptf::Signature::new();

        let mut kpv = Vec::<u8>::new();
        kpv.append(&mut kp.get_public_key());
        kpv.append(&mut kp.get_secret_key());
        let mut sgv = Vec::<u8>::new();
        sgv.append(&mut signature.get_public_key());
        sgv.append(&mut signature.get_secret_key());

        let (cert, key) = match self.cert_service.sign_player_cert(player_name) {
            Ok((cert, key)) => (cert, key),
            Err(e) => {
                tracing::error!(
                    "Failed to sign certificate for {}: {}",
                    player_name,
                    e.to_string()
                );
                return;
            }
        };

        // Generate gamerpic for Hytale players if we have their UUID
        let gamerpic = match (game_type, player_uuid) {
            (Game::Hytale, Some(uuid)) => Some(Self::hytale_gamerpic_from_uuid(uuid)),
            _ => None,
        };

        let p = player::ActiveModel {
            id: ActiveValue::NotSet,
            gamertag: ActiveValue::Set(Some(player_name.to_string())),
            gamerpic: ActiveValue::Set(gamerpic),
            certificate: ActiveValue::Set(cert.pem()),
            certificate_key: ActiveValue::Set(key.serialize_pem()),
            banished: ActiveValue::Set(false),
            keypair: ActiveValue::Set(kpv),
            signature: ActiveValue::Set(sgv),
            created_at: ActiveValue::Set(Utc::now().timestamp() as u32),
            updated_at: ActiveValue::Set(Utc::now().timestamp() as u32),
            game: ActiveValue::Set(game_type.clone()),
        };

        match p.insert(self.db.as_ref()).await {
            Ok(inserted) => {
                tracing::info!("Created player record for: {}", player_name);
                self.cache.insert(player_name.to_string());

                // Store platform UUID identity if provided
                if let Some(uuid) = player_uuid {
                    self.store_platform_uuid(inserted.id, uuid, game_type)
                        .await;
                }
            }
            Err(e) => {
                tracing::error!(
                    "Unable to insert player {} into database: {}",
                    player_name,
                    e.to_string()
                );
            }
        }
    }

    /// INSERT OR IGNORE a platform UUID into the player_identity table.
    async fn store_platform_uuid(&self, player_id: i32, uuid: &str, game_type: &Game) {
        let now = Utc::now().timestamp() as u32;
        let identity = player_identity::ActiveModel {
            id: ActiveValue::NotSet,
            player_id: ActiveValue::Set(player_id),
            alias: ActiveValue::Set(uuid.to_string()),
            game: ActiveValue::Set(game_type.clone()),
            alias_type: ActiveValue::Set("platform_uuid".to_string()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };

        if let Err(e) = player_identity::Entity::insert(identity)
            .on_conflict(
                OnConflict::columns([
                    player_identity::Column::Alias,
                    player_identity::Column::Game,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec_without_returning(self.db.as_ref())
            .await
        {
            tracing::error!(
                "Failed to store platform UUID for player_id {}: {}",
                player_id,
                e
            );
        }
    }

    /// Update gamerpic for an existing Hytale player that has none.
    async fn generate_hytale_gamerpic(&self, player_id: i32, uuid: &str) {
        let gamerpic = Self::hytale_gamerpic_from_uuid(uuid);

        let mut model: player::ActiveModel = Default::default();
        model.id = ActiveValue::Unchanged(player_id);
        model.gamerpic = ActiveValue::Set(Some(gamerpic));

        if let Err(e) = model.update(self.db.as_ref()).await {
            tracing::error!(
                "Failed to update gamerpic for player_id {}: {}",
                player_id,
                e
            );
        }
    }

    fn hytale_gamerpic_from_uuid(uuid: &str) -> String {
        let avatar_url = format!("https://crafthead.net/hytale/avatar/{}", uuid);
        general_purpose::STANDARD.encode(&avatar_url)
    }
}
