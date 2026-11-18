//! Player registration service

use common::curia;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use common::Game;
use common::ncryptflib as ncryptf;
use common::ncryptflib::rocket::Utc;
use common::traits::player_data::PlayerData;
use entity::{player, player_identity};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter,
};

use crate::services::CertificateService;

mod bound_uuid_cache;
mod cache;

pub use bound_uuid_cache::BoundUuidCache;
pub use cache::RegisteredPlayersCache;

/// Service for player registration logic.
/// Creates new player records in the database with certificates.
/// Shared between HTTP routes and FFI to ensure players are registered
/// regardless of how position updates are received.
#[derive(Clone)]
pub struct PlayerRegistrarService {
    db: Arc<DatabaseConnection>,
    cert_service: Arc<CertificateService>,
    cache: RegisteredPlayersCache,
    bound_uuids: BoundUuidCache,
    // Serializes check-and-create. The FFI path spawns a registration per position tick, and
    // the gamertag index is not unique, so two overlapping first ticks would insert twice.
    register_lock: Arc<tokio::sync::Mutex<()>>,
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
            bound_uuids: BoundUuidCache::new(),
            register_lock: Arc::new(tokio::sync::Mutex::new(())),
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
    /// * `game_type` - The game type
    pub async fn process_players(&self, players: &[common::PlayerEnum], game_type: Game) {
        // Build name → UUID map for players that have a platform UUID
        let uuid_map: HashMap<String, String> = players
            .iter()
            .filter_map(|p| {
                p.get_player_uuid()
                    .map(|uuid| (p.get_name().to_string(), uuid.to_string()))
            })
            .collect();

        self.bind_unseen_uuids(&uuid_map, &game_type).await;

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

        let _registering = self.register_lock.lock().await;
        let players_to_check: Vec<String> = players_to_check
            .into_iter()
            .filter(|name| !self.cache.contains(name))
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
                    if let Err(e) = self.create_unlocked(&player_name, &game_type, uuid).await {
                        curia::error!("failed to register a player", { "gamertag": player_name.to_string(), "error": e.to_string() });
                    }
                }
            }
            Err(e) => {
                curia::error!("Failed to query database: {}", e.to_string());
            }
        }
    }

    /// The player registered under this exact gamertag for `game`.
    pub async fn find(&self, gamertag: &str, game: &Game) -> Result<Option<player::Model>, DbErr> {
        player::Entity::find()
            .filter(player::Column::Gamertag.eq(gamertag))
            .filter(player::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
    }

    /// Returns the existing player for (gamertag, game), or mints its certificate and
    /// keypairs and inserts it. A `player_uuid` is recorded either way.
    ///
    /// `idx_player_gamertag_game` is not unique, so the check and the insert run under this
    /// service's lock. Every caller in the server process must share one instance for that
    /// to hold; a second instance, such as a CLI command's, can still insert a duplicate.
    pub async fn create(
        &self,
        player_name: &str,
        game_type: &Game,
        player_uuid: Option<&str>,
    ) -> Result<player::Model, anyhow::Error> {
        let _registering = self.register_lock.lock().await;
        self.create_unlocked(player_name, game_type, player_uuid)
            .await
    }

    // The body of `create`, for a caller that already holds `register_lock`.
    async fn create_unlocked(
        &self,
        player_name: &str,
        game_type: &Game,
        player_uuid: Option<&str>,
    ) -> Result<player::Model, anyhow::Error> {
        let existing = self
            .find(player_name, game_type)
            .await
            .map_err(|e| anyhow::anyhow!("failed to look up player: {}", e))?;

        if let Some(existing) = existing {
            self.cache.insert(player_name.to_string());
            if let Some(uuid) = player_uuid {
                self.store_platform_uuid(existing.id, uuid, game_type).await;
            }
            return Ok(existing);
        }

        let kp = ncryptf::Keypair::new();
        let signature = ncryptf::Signature::new();

        let mut kpv = Vec::<u8>::new();
        kpv.append(&mut kp.get_public_key());
        kpv.append(&mut kp.get_secret_key());
        let mut sgv = Vec::<u8>::new();
        sgv.append(&mut signature.get_public_key());
        sgv.append(&mut signature.get_secret_key());

        let (cert, key) = self
            .cert_service
            .sign_player_cert(player_name, game_type)
            .map_err(|e| {
                curia::error!(
                    "Failed to sign certificate for {}: {}",
                    player_name,
                    e.to_string()
                );
                anyhow::anyhow!("failed to sign certificate: {}", e)
            })?;

        let gamerpic: Option<String> = None;

        let p = player::ActiveModel {
            id: ActiveValue::NotSet,
            gamertag: ActiveValue::Set(Some(player_name.to_string())),
            gamerpic: ActiveValue::Set(gamerpic),
            certificate: ActiveValue::Set(cert.pem()),
            certificate_key: ActiveValue::Set(key.serialize_pem()),
            banished: ActiveValue::Set(false),
            keypair: ActiveValue::Set(kpv),
            signature: ActiveValue::Set(sgv),
            created_at: ActiveValue::Set(Utc::now().timestamp()),
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            game: ActiveValue::Set(game_type.clone()),
        };

        let inserted = p.insert(self.db.as_ref()).await.map_err(|e| {
            curia::error!(
                "Unable to insert player {} into database: {}",
                player_name,
                e.to_string()
            );
            anyhow::anyhow!("failed to insert player: {}", e)
        })?;

        curia::info!("Created player record for: {}", player_name);
        self.cache.insert(player_name.to_string());

        // Store platform UUID identity if provided
        if let Some(uuid) = player_uuid {
            self.store_platform_uuid(inserted.id, uuid, game_type).await;
        }

        Ok(inserted)
    }

    // Binds the platform UUID of every entry whose name is already registered and whose
    // UUID this process has not written yet. A name is registered once, but more than one
    // player can arrive under it: a Java account and a Bedrock player whose gamertag is the
    // same text. Each UUID is bound, so a later ownership check sees both.
    async fn bind_unseen_uuids(&self, uuid_map: &HashMap<String, String>, game_type: &Game) {
        let unseen: HashMap<&String, &String> = uuid_map
            .iter()
            .filter(|(name, uuid)| self.cache.contains(name) && !self.bound_uuids.contains(uuid))
            .collect();
        if unseen.is_empty() {
            return;
        }

        let names: Vec<String> = unseen.keys().map(|name| (*name).clone()).collect();
        match player::Entity::find()
            .filter(player::Column::Gamertag.is_in(names))
            .filter(player::Column::Game.eq(game_type.clone()))
            .all(self.db.as_ref())
            .await
        {
            Ok(records) => {
                for record in records {
                    let Some(uuid) = record.gamertag.as_ref().and_then(|name| unseen.get(name))
                    else {
                        continue;
                    };
                    self.store_platform_uuid(record.id, uuid, game_type).await;
                }
            }
            Err(e) => {
                curia::error!("failed to look up players for a platform uuid", { "error": e.to_string() });
            }
        }
    }

    /// INSERT OR IGNORE a platform UUID into the player_identity table.
    async fn store_platform_uuid(&self, player_id: i32, uuid: &str, game_type: &Game) {
        let uuid = uuid.to_ascii_lowercase();
        let uuid = uuid.as_str();
        let now = Utc::now().timestamp();
        let identity = player_identity::ActiveModel {
            id: ActiveValue::NotSet,
            player_id: ActiveValue::Set(player_id),
            alias: ActiveValue::Set(uuid.to_string()),
            game: ActiveValue::Set(game_type.clone()),
            alias_type: ActiveValue::Set("platform_uuid".to_string()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };

        match player_identity::Entity::insert(identity)
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
            // Remembered only after the write, so a failed write is tried again.
            Ok(_) => self.bound_uuids.insert(uuid),
            Err(e) => {
                curia::error!(
                    "Failed to store platform UUID for player_id {}: {}",
                    player_id,
                    e
                );
            }
        }
    }
}
