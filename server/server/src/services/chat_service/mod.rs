mod quic_sink;
mod registry;
mod rejection;
mod sink;

pub use quic_sink::QuicChatSink;
pub use registry::{ChatSocket, ChatSocketRegistry};
pub use rejection::ChatRejection;
pub use sink::ChatSink;

use std::sync::{Arc, OnceLock, RwLock};

use common::{Game, PlayerEnum};
use common::structs::chat::ChatFrame;
use common::structs::packet::{ChatMessagePacket, ChatOrigin};
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait, sea_query::OnConflict};
use tokio::sync::mpsc;

use crate::services::PlayerIdentityService;

/// The net-mode chat hub.
///
/// Holds one mod connection per world, fans reported lines out to every registered sink, and
/// records which worlds a player has been seen in. Message content is never stored.
///
/// Its dependencies arrive after construction rather than through the constructor, matching
/// `CacheManager::set_webhook_receiver` and `ConnectionRegistry::set_peer_manager`. That is
/// the house pattern, and it is what lets the unit tests exercise routing without a database.
pub struct ChatService {
    registry: ChatSocketRegistry,
    sinks: RwLock<Vec<Arc<dyn ChatSink>>>,
    // The raw presence handle rather than the whole `PlayerCache`: the service only needs
    // to answer "which world is this identity in", and `cache_manager` stays private.
    players: OnceLock<Arc<moka::future::Cache<String, PlayerEnum>>>,
    db: OnceLock<Arc<DatabaseConnection>>,
    identities: OnceLock<Arc<PlayerIdentityService>>,
}

impl ChatService {
    pub fn new() -> Self {
        Self {
            registry: ChatSocketRegistry::new(),
            sinks: RwLock::new(Vec::new()),
            players: OnceLock::new(),
            db: OnceLock::new(),
            identities: OnceLock::new(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn set_players(&self, players: Arc<moka::future::Cache<String, PlayerEnum>>) {
        let _ = self.players.set(players);
    }

    pub fn set_db(&self, db: Arc<DatabaseConnection>) {
        let _ = self.db.set(db);
    }

    pub fn set_identities(&self, identities: Arc<PlayerIdentityService>) {
        let _ = self.identities.set(identities);
    }

    pub fn add_sink(&self, sink: Arc<dyn ChatSink>) {
        if let Ok(mut sinks) = self.sinks.write() {
            sinks.push(sink);
        }
    }

    pub fn register(
        &self,
        world_uuid: String,
        world_name: String,
        tx: mpsc::Sender<String>,
    ) -> Option<mpsc::Sender<String>> {
        tracing::info!(world = %world_uuid, name = %world_name, "chat channel registered");
        self.registry.register(world_uuid, world_name, tx)
    }

    /// Registers one socket under every id its room spans.
    pub fn register_room(
        &self,
        worlds: &[String],
        world_name: String,
        tx: mpsc::Sender<String>,
    ) -> Vec<mpsc::Sender<String>> {
        tracing::info!(
            worlds = ?worlds,
            name = %world_name,
            "chat channel registered"
        );
        self.registry.register_room(worlds, world_name, tx)
    }

    pub fn unregister(&self, world_uuid: &str) {
        tracing::info!(world = %world_uuid, "chat channel unregistered");
        self.registry.unregister(world_uuid);
    }

    pub fn is_available(&self, world_uuid: &str) -> bool {
        self.registry.contains(world_uuid)
    }

    pub fn world_name(&self, world_uuid: &str) -> Option<String> {
        self.registry.world_name(world_uuid)
    }

    /// A line a mod reported.
    ///
    /// `worlds` is every id this chat room spans — one on BDS, one per dimension on Paper and
    /// Fabric. The line is delivered under each so that everyone in the room hears it whatever
    /// dimension they are standing in, but history is recorded only under the canonical id:
    /// the picker lists rooms, and three entries for one server would be three lies.
    pub async fn on_game_chat(&self, worlds: &[String], author: String, text: String) {
        let Some(canonical) = worlds.first() else {
            return;
        };
        self.record_history(canonical, &author).await;

        for world_uuid in worlds {
            let packet = ChatMessagePacket::new(
                Some(author.clone()),
                text.clone(),
                Some(world_uuid.clone()),
                ChatOrigin::Game,
            );
            self.fan_out(world_uuid, &packet);
        }
    }

    /// A line composed in the app.
    ///
    /// Both halves happen here: the mod is told to broadcast it in game, and clients are told
    /// directly. Waiting for the mod to echo it would never work — a programmatic broadcast
    /// does not fire the mod's own chat listener.
    pub async fn on_app_send(
        &self,
        author: &str,
        world_uuid: &str,
        text: String,
    ) -> Result<(), ChatRejection> {
        // Resolve to the room rather than the id. On Paper and Fabric the id names a
        // dimension, and the room is all of them: a player standing in the nether is in the
        // same room as the overworld id the app addressed.
        let room = self
            .registry
            .room(world_uuid)
            .ok_or(ChatRejection::NoChannel)?;

        // The sender's live world beats the world they named — they may have been transferred
        // while the app still held the older target. Compared against the room, so changing
        // dimension is not mistaken for changing server.
        if let Some(current) = self.current_world_of(author).await {
            if !room.contains(&current) {
                return Err(ChatRejection::WrongWorld {
                    current: Some(current),
                });
            }
        }

        let tx = self
            .registry
            .sender(world_uuid)
            .ok_or(ChatRejection::NoChannel)?;

        // The gamertag, not the certificate CN. `minecraft:Alaydriem` is how the voice plane
        // keys identity; it is not what belongs in a chat line.
        let display = Self::display_name(author);

        let frame = ChatFrame::Say {
            author: display.clone(),
            text: text.clone(),
        };
        let body = serde_json::to_string(&frame).map_err(|_| ChatRejection::NoChannel)?;
        tx.try_send(body).map_err(|_| ChatRejection::NoChannel)?;

        // Delivered to the whole room, exactly as a typed line is. Delivering only under the
        // id the client named would hide it from anyone in another dimension.
        for id in room.iter() {
            let packet = ChatMessagePacket::new(
                Some(display.clone()),
                text.clone(),
                Some(id.clone()),
                ChatOrigin::App,
            );
            self.fan_out(id, &packet);
        }
        Ok(())
    }

    fn fan_out(&self, world_uuid: &str, packet: &ChatMessagePacket) {
        if let Ok(sinks) = self.sinks.read() {
            for sink in sinks.iter() {
                sink.deliver(world_uuid, packet);
            }
        }
    }

    /// `None` when the player is not in the game at all, which is the off-game case the
    /// picker exists for. Only a live mismatch is a rejection.
    async fn current_world_of(&self, identity: &str) -> Option<String> {
        let players = self.players.get()?;
        match players.get(identity).await? {
            PlayerEnum::Minecraft(mc) => mc.world_uuid.clone(),
            _ => None,
        }
    }

    /// Membership only — never message content. This is what the off-game picker lists.
    async fn record_history(&self, world_uuid: &str, author: &str) {
        let (Some(db), Some(identities)) = (self.db.get(), self.identities.get()) else {
            // Unset in the unit tests, which is what keeps them database-free.
            return;
        };

        let Some(player_id) = identities
            .find_player_id_by_gamertag(author, &Game::Minecraft)
            .await
        else {
            return;
        };

        let now = common::ncryptflib::rocket::Utc::now().timestamp();
        let name = self
            .world_name(world_uuid)
            .unwrap_or_else(|| world_uuid.to_string());

        let row = entity::player_world::ActiveModel {
            player_id: ActiveValue::Set(player_id),
            world_uuid: ActiveValue::Set(world_uuid.to_string()),
            world_name: ActiveValue::Set(name),
            last_seen: ActiveValue::Set(now),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };

        // Conflicts on the unique (player_id, world_uuid) index. A player is seen in a world
        // constantly, so this runs often and must not accumulate rows.
        let result = entity::player_world::Entity::insert(row)
            .on_conflict(
                OnConflict::columns([
                    entity::player_world::Column::PlayerId,
                    entity::player_world::Column::WorldUuid,
                ])
                .update_columns([
                    entity::player_world::Column::WorldName,
                    entity::player_world::Column::LastSeen,
                    entity::player_world::Column::UpdatedAt,
                ])
                .to_owned(),
            )
            .exec(db.as_ref())
            .await;

        if let Err(e) = result {
            tracing::warn!(world = %world_uuid, "failed to record chat world history: {}", e);
        }
    }

    /// Strips the `game:` prefix the voice plane keys on.
    fn display_name(identity: &str) -> String {
        identity
            .split_once(':')
            .map(|(_, name)| name.to_string())
            .unwrap_or_else(|| identity.to_string())
    }
}

impl Default for ChatService {
    fn default() -> Self {
        Self::new()
    }
}
