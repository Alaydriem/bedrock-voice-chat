use common::curia;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::traits::player_data::PlayerData;
use common::{Game, PlayerEnum};
use entity::{player, player_identity};
use moka::future::Cache;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

mod alias_claim;

pub use alias_claim::AliasClaim;

/// Service for resolving player identity aliases.
/// Maps in-game names (e.g., Minecraft Java usernames, Floodgate-prefixed names)
/// to canonical gamertags (Xbox Live gamertags).
#[derive(Clone)]
pub struct PlayerIdentityService {
    db: Arc<DatabaseConnection>,
    alias_cache: Cache<(String, Game), String>,
    // Claims refused, or not yet recordable, keyed (alias, gamertag, game). Positions arrive
    // several times a second; without this a collision costs queries and a warning per tick.
    refused_claims: Cache<(String, String, Game), ()>,
    // (name, dropped uuid) pairs already warned about.
    warned_duplicates: Cache<(String, String), ()>,
    // Whether an entry (reported name, platform uuid, game) follows the alias on its name.
    remap_decisions: Cache<(String, String, Game), bool>,
}

impl PlayerIdentityService {
    // Floodgate stores an unlinked Bedrock player under UUID(0, xuid). Such a UUID proves
    // the reported name is that player's Xbox gamertag.
    const FLOODGATE_UUID_PREFIX: &'static str = "00000000-0000-0000-";
    const PLATFORM_UUID: &'static str = "platform_uuid";

    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let alias_cache = Cache::builder()
            .time_to_live(Duration::from_secs(86400))
            .max_capacity(512)
            .build();
        let refused_claims = Cache::builder()
            .time_to_live(Duration::from_secs(600))
            .max_capacity(1024)
            .build();
        let warned_duplicates = Cache::builder()
            .time_to_live(Duration::from_secs(3600))
            .max_capacity(1024)
            .build();

        let remap_decisions = Cache::builder()
            .time_to_live(Duration::from_secs(600))
            .max_capacity(1024)
            .build();

        Self {
            db,
            alias_cache,
            refused_claims,
            warned_duplicates,
            remap_decisions,
        }
    }

    /// Name every roster entry by the gamertag its player signs in with, before the roster is
    /// registered or broadcast.
    ///
    /// An entry carrying `alternative_identity` takes it as its name, and the name it was
    /// reported under is returned as an [`AliasClaim`] for [`Self::record_alias_claims`] once
    /// the gamertag is registered. Other entries follow a known alias, but only when their
    /// platform UUID belongs to the alias's target or to the record the alias names, so a
    /// stranger who shares a name is never moved onto someone else's gamertag. When two
    /// entries end with one name, the one whose name is an Xbox gamertag keeps it and the
    /// other is removed from this roster.
    pub async fn canonicalize_roster(
        &self,
        players: &mut Vec<PlayerEnum>,
        game: &Game,
    ) -> Vec<AliasClaim> {
        let mut claims = Vec::new();
        let mut verified = vec![false; players.len()];

        for (index, player) in players.iter_mut().enumerate() {
            let Some(gamertag) = player
                .get_alternative_identity()
                .filter(|alt| !alt.is_empty())
                .map(String::from)
            else {
                continue;
            };
            player.clear_alternative_identity();
            verified[index] = true;

            let reported = player.get_name().to_string();
            if gamertag != reported {
                claims.push(AliasClaim::new(&reported, &gamertag, player.get_player_uuid()));
                player.set_name(gamertag);
            }
        }

        for (index, player) in players.iter_mut().enumerate() {
            if verified[index] {
                continue;
            }

            let reported = player.get_name().to_string();
            let target = self.resolve_name(&reported, game).await;
            if target == reported {
                continue;
            }

            let follows = match player.get_player_uuid() {
                None => true,
                Some(uuid) => self.remap_allowed(uuid, &target, &reported, game).await,
            };
            if follows {
                player.set_name(target);
            }
        }

        self.drop_duplicate_names(players, &verified).await;
        claims
    }

    /// Record the aliases [`Self::canonicalize_roster`] claimed, now that the roster's
    /// gamertags are registered.
    ///
    /// A claim is refused when the alias names another player's record, or already points at
    /// another player, and the claimant's platform UUID does not prove the record is theirs.
    pub async fn record_alias_claims(&self, claims: &[AliasClaim], game: &Game) {
        for claim in claims {
            let cached = self
                .alias_cache
                .get(&(claim.alias.clone(), game.clone()))
                .await;
            if cached.as_deref() == Some(claim.gamertag.as_str()) {
                continue;
            }

            let key = (claim.alias.clone(), claim.gamertag.clone(), game.clone());
            if self.refused_claims.get(&key).await.is_some() {
                continue;
            }

            let Some(target_id) = self.find_player_id_by_gamertag(&claim.gamertag, game).await
            else {
                self.refused_claims.insert(key, ()).await;
                continue;
            };

            let claimant = match claim.player_uuid.as_deref() {
                Some(uuid) => self.platform_uuid_owner(uuid, game).await,
                None => None,
            };

            if let Some(alias_record) = self.find_player_id_by_gamertag(&claim.alias, game).await {
                if claimant != Some(alias_record) {
                    self.refuse(key, claim, "the alias is another player's record").await;
                    continue;
                }
            }

            if let Some(holder) = self.alias_holder(&claim.alias, game).await {
                if holder != target_id && claimant != Some(holder) {
                    self.refuse(key, claim, "the alias belongs to another player").await;
                    continue;
                }
            }

            if let Err(e) = self
                .create_alias(target_id, &claim.alias, game, "floodgate")
                .await
            {
                curia::warn!("failed to record a floodgate alias", { "alias": claim.alias.clone(), "gamertag": claim.gamertag.clone(), "error": e.to_string() });
            }
        }
    }

    async fn refuse(&self, key: (String, String, Game), claim: &AliasClaim, reason: &str) {
        self.refused_claims.insert(key, ()).await;
        curia::warn!("refusing a floodgate alias held by another player", { "alias": claim.alias.clone(), "gamertag": claim.gamertag.clone(), "reason": reason.to_string() });
    }

    // Removes every entry whose final name an earlier-kept entry already holds, preferring
    // the entry whose name is proven to be an Xbox gamertag.
    async fn drop_duplicate_names(&self, players: &mut Vec<PlayerEnum>, verified: &[bool]) {
        let xbox = |index: usize, player: &PlayerEnum| {
            verified[index]
                || player
                    .get_player_uuid()
                    .is_some_and(|uuid| uuid.starts_with(Self::FLOODGATE_UUID_PREFIX))
        };

        let mut owner: HashMap<String, usize> = HashMap::new();
        let mut keep = vec![true; players.len()];
        for (index, player) in players.iter().enumerate() {
            let name = player.get_name().to_string();
            let Some(&held) = owner.get(&name) else {
                owner.insert(name, index);
                continue;
            };

            let (kept, dropped) = if xbox(index, player) && !xbox(held, &players[held]) {
                owner.insert(name.clone(), index);
                (index, held)
            } else {
                (held, index)
            };
            keep[dropped] = false;

            let dropped_uuid = players[dropped].get_player_uuid().unwrap_or("").to_string();
            let warn_key = (name.clone(), dropped_uuid.clone());
            if self.warned_duplicates.get(&warn_key).await.is_none() {
                self.warned_duplicates.insert(warn_key, ()).await;
                curia::warn!("two players report the same name; keeping the Xbox gamertag's owner", { "gamertag": name, "kept_uuid": players[kept].get_player_uuid().unwrap_or("").to_string(), "dropped_uuid": dropped_uuid });
            }
        }

        let mut index = 0;
        players.retain(|_| {
            let kept = keep[index];
            index += 1;
            kept
        });
    }

    // Whether an entry reported as `reported` with `uuid` is the player alias `reported` →
    // `target` belongs to. Proven when the UUID is bound to the target's record, or to the
    // record named `reported` and bound before the alias existed: that record was the
    // claimant's own when the alias was granted. A UUID bound to that record afterwards
    // belongs to someone who registered under the name later, and proves nothing.
    // Decisions are cached; positions arrive several times a second.
    async fn remap_allowed(&self, uuid: &str, target: &str, reported: &str, game: &Game) -> bool {
        let key = (reported.to_string(), uuid.to_string(), game.clone());
        if let Some(decision) = self.remap_decisions.get(&key).await {
            return decision;
        }

        let decision = match self.platform_uuid_row(uuid, game).await {
            None => false,
            Some(uuid_row) => {
                if self.find_player_id_by_gamertag(target, game).await == Some(uuid_row.player_id) {
                    true
                } else if self.find_player_id_by_gamertag(reported, game).await
                    == Some(uuid_row.player_id)
                {
                    self.alias_row(reported, game)
                        .await
                        .is_some_and(|alias| uuid_row.id < alias.id)
                } else {
                    false
                }
            }
        };

        self.remap_decisions.insert(key, decision).await;
        decision
    }

    async fn platform_uuid_owner(&self, uuid: &str, game: &Game) -> Option<i32> {
        self.platform_uuid_row(uuid, game).await.map(|r| r.player_id)
    }

    async fn platform_uuid_row(&self, uuid: &str, game: &Game) -> Option<player_identity::Model> {
        match player_identity::Entity::find()
            .filter(player_identity::Column::Alias.eq(uuid))
            .filter(player_identity::Column::AliasType.eq(Self::PLATFORM_UUID))
            .filter(player_identity::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
        {
            Ok(row) => row,
            Err(e) => {
                curia::error!("failed to look up a platform uuid", { "error": e.to_string() });
                None
            }
        }
    }

    // The player an existing name alias points at, ignoring platform UUID rows.
    async fn alias_holder(&self, alias: &str, game: &Game) -> Option<i32> {
        self.alias_row(alias, game).await.map(|r| r.player_id)
    }

    async fn alias_row(&self, alias: &str, game: &Game) -> Option<player_identity::Model> {
        match player_identity::Entity::find()
            .filter(player_identity::Column::Alias.eq(alias))
            .filter(player_identity::Column::AliasType.ne(Self::PLATFORM_UUID))
            .filter(player_identity::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
        {
            Ok(row) => row,
            Err(e) => {
                curia::error!("failed to look up an alias", { "error": e.to_string() });
                None
            }
        }
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
                    self.alias_cache.insert(cache_key, gt.clone()).await;
                    return gt.clone();
                }
            }
            Ok(None) => {}
            Err(e) => {
                curia::error!("Failed to query player_identity: {}", e);
            }
        }

        in_game_name.to_string()
    }

    /// Bulk resolve in-game names to canonical gamertags, mutating player names in place.
    pub async fn resolve_and_remap_players(&self, players: &mut Vec<PlayerEnum>, game: &Game) {
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
        let now = common::ncryptflib::rocket::Utc::now().timestamp();

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

        curia::debug!(
            "Created/updated identity alias: {} -> player_id {} ({:?}, type: {})",
            alias,
            player_id,
            game,
            alias_type
        );

        Ok(())
    }

    /// Look up a player's ID by their canonical gamertag.
    pub async fn find_player_id_by_gamertag(&self, gamertag: &str, game: &Game) -> Option<i32> {
        match player::Entity::find()
            .filter(player::Column::Gamertag.eq(gamertag))
            .filter(player::Column::Game.eq(game.clone()))
            .one(self.db.as_ref())
            .await
        {
            Ok(Some(p)) => Some(p.id),
            Ok(None) => None,
            Err(e) => {
                curia::error!("Failed to find player by gamertag: {}", e);
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
                curia::error!(
                    "Failed to lookup gamertag for player_id {}: {}",
                    player_id,
                    e
                );
                None
            }
        }
    }
}
