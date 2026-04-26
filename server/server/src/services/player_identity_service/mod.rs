use std::sync::Arc;
use std::time::Duration;

use common::traits::player_data::PlayerData;
use common::{Game, PlayerEnum};
use entity::{player, player_identity};
use moka::future::Cache;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

/// Service for resolving player identity aliases.
/// Maps in-game names (e.g., Minecraft Java usernames, Floodgate-prefixed names)
/// to canonical gamertags (Xbox Live gamertags).
#[derive(Clone)]
pub struct PlayerIdentityService {
    db: Arc<DatabaseConnection>,
    alias_cache: Cache<(String, Game), String>,
}

impl PlayerIdentityService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let alias_cache = Cache::builder()
            .time_to_live(Duration::from_secs(86400))
            .max_capacity(512)
            .build();

        Self { db, alias_cache }
    }

    /// Resolve an in-game name to its canonical gamertag.
    /// Returns the original name unchanged if no alias exists (backward compatible).
    pub async fn resolve_name(&self, in_game_name: &str, game: &Game) -> String {
        let cache_key = (in_game_name.to_string(), game.clone());

        if let Some(gamertag) = self.alias_cache.get(&cache_key).await {
            return gamertag;
        }

        match player_identity::Entity::find()
            .filter(player_identity::Column::Alias.eq(in_game_name))
            .filter(player_identity::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
        {
            Ok(Some(identity)) => {
                let gamertag = self.lookup_gamertag(identity.player_id, game).await;
                if let Some(ref gt) = gamertag {
                    self.alias_cache
                        .insert(cache_key, gt.clone())
                        .await;
                    return gt.clone();
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Failed to query player_identity: {}", e);
            }
        }

        in_game_name.to_string()
    }

    /// Resolve players by platform UUID alias before the name-based pass.
    /// If a `platform_uuid` alias exists for the player's UUID, remaps the player's
    /// name to the canonical gamertag and records the current Java username as a
    /// `minecraft_services` alias if it has drifted.
    pub async fn resolve_uuid_and_remap_players(
        &self,
        players: &mut Vec<common::PlayerEnum>,
        game: &Game,
    ) {
        for player in players.iter_mut() {
            let uuid = match player.get_player_uuid() {
                Some(u) => u.to_string(),
                None => continue,
            };
            if uuid.is_empty() {
                continue;
            }
            if let Some(player_id) = self.find_player_id_by_alias(&uuid, "platform_uuid", game).await {
                if let Some(canonical) = self.lookup_gamertag(player_id, game).await {
                    let current_name = player.get_name().to_string();
                    if canonical != current_name {
                        let _ = self
                            .create_alias(player_id, &current_name, game, "minecraft_services")
                            .await;
                        player.set_name(canonical);
                    }
                }
            }
        }
    }

    /// Bulk resolve in-game names to canonical gamertags, mutating player names in place.
    pub async fn resolve_and_remap_players(
        &self,
        players: &mut Vec<PlayerEnum>,
        game: &Game,
    ) {
        for player in players.iter_mut() {
            let current_name = player.get_name().to_string();
            let resolved = self.resolve_name(&current_name, game).await;
            if resolved != current_name {
                player.set_name(resolved);
            }
        }
    }

    /// Create or update an alias mapping.
    pub async fn create_alias(
        &self,
        player_id: i32,
        alias: &str,
        game: &Game,
        alias_type: &str,
    ) -> Result<(), anyhow::Error> {
        let now = common::ncryptflib::rocket::Utc::now().timestamp() as u32;

        // Check if alias already exists
        let existing = player_identity::Entity::find()
            .filter(player_identity::Column::Alias.eq(alias))
            .filter(player_identity::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await?;

        match existing {
            Some(record) => {
                // Update if the player_id changed (e.g., name was transferred)
                if record.player_id != player_id {
                    let mut active: player_identity::ActiveModel = record.into();
                    active.player_id = ActiveValue::Set(player_id);
                    active.alias_type = ActiveValue::Set(alias_type.to_string());
                    active.save(self.db.as_ref()).await?;
                }
            }
            None => {
                let new_identity = player_identity::ActiveModel {
                    id: ActiveValue::NotSet,
                    player_id: ActiveValue::Set(player_id),
                    alias: ActiveValue::Set(alias.to_string()),
                    game: ActiveValue::Set(game.clone()),
                    alias_type: ActiveValue::Set(alias_type.to_string()),
                    created_at: ActiveValue::Set(now),
                    updated_at: ActiveValue::Set(now),
                };
                new_identity.insert(self.db.as_ref()).await?;
            }
        }

        // Update cache: alias -> gamertag
        if let Some(gamertag) = self.lookup_gamertag(player_id, game).await {
            self.alias_cache
                .insert((alias.to_string(), game.clone()), gamertag)
                .await;
        }

        tracing::debug!(
            "Created/updated identity alias: {} -> player_id {} ({:?}, type: {})",
            alias,
            player_id,
            game,
            alias_type
        );

        Ok(())
    }

    /// Look up a player's ID by their canonical gamertag.
    pub async fn find_player_id_by_gamertag(
        &self,
        gamertag: &str,
        game: &Game,
    ) -> Option<i32> {
        match player::Entity::find()
            .filter(player::Column::Gamertag.eq(gamertag))
            .filter(player::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
        {
            Ok(Some(p)) => Some(p.id),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to find player by gamertag: {}", e);
                None
            }
        }
    }

    pub async fn find_player_id_by_alias(
        &self,
        alias: &str,
        alias_type: &str,
        game: &Game,
    ) -> Option<i32> {
        match player_identity::Entity::find()
            .filter(player_identity::Column::Alias.eq(alias))
            .filter(player_identity::Column::AliasType.eq(alias_type))
            .filter(player_identity::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
        {
            Ok(Some(record)) => Some(record.player_id),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to find player_id by alias: {}", e);
                None
            }
        }
    }

    /// Look up a gamertag by player ID.
    async fn lookup_gamertag(&self, player_id: i32, game: &Game) -> Option<String> {
        match player::Entity::find_by_id(player_id)
            .filter(player::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
        {
            Ok(Some(p)) => p.gamertag,
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to lookup gamertag for player_id {}: {}", player_id, e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sea_orm::{DatabaseBackend, MockDatabase};

    use super::*;

    #[test]
    fn cache_key_equality() {
        let key1 = ("CoolBuilder42".to_string(), Game::Minecraft);
        let key2 = ("CoolBuilder42".to_string(), Game::Minecraft);
        assert_eq!(key1, key2);

        let key3 = ("CoolBuilder42".to_string(), Game::Hytale);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn find_player_id_by_alias_returns_some_when_match() {
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results([vec![player_identity::Model {
                id: 1,
                player_id: 42,
                alias: "uuid-1".into(),
                game: Game::Minecraft,
                alias_type: "platform_uuid".into(),
                created_at: 0,
                updated_at: 0,
            }]])
            .into_connection();

        let svc = PlayerIdentityService::new(Arc::new(db));
        let id = svc
            .find_player_id_by_alias("uuid-1", "platform_uuid", &Game::Minecraft)
            .await;
        assert_eq!(id, Some(42));
    }

    #[tokio::test]
    async fn find_player_id_by_alias_returns_none_on_miss() {
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results([Vec::<player_identity::Model>::new()])
            .into_connection();

        let svc = PlayerIdentityService::new(Arc::new(db));
        let id = svc
            .find_player_id_by_alias("missing", "platform_uuid", &Game::Minecraft)
            .await;
        assert!(id.is_none());
    }
}
