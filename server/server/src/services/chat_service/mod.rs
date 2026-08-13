mod quic_sink;
mod registry;
mod sink;

pub use quic_sink::QuicChatSink;
pub use registry::{ChatSocket, ChatSocketRegistry};
pub use sink::ChatSink;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use common::{Game, PlayerEnum};
use common::structs::chat::ChatFrame;
use common::errors::ChatRejection;
use common::structs::packet::{ChatMessagePacket, ChatOrigin, ChatRejectedPacket};
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
    /// Throttles the history upsert. A player is seen in a world four times a second, and the
    /// row only needs to be fresh enough to order a picker.
    recently_recorded: moka::future::Cache<(String, String), ()>,
    next_socket_id: AtomicU64,
}

impl ChatService {
    pub fn new() -> Self {
        Self {
            registry: ChatSocketRegistry::new(),
            sinks: RwLock::new(Vec::new()),
            players: OnceLock::new(),
            db: OnceLock::new(),
            identities: OnceLock::new(),
            recently_recorded: moka::future::Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .max_capacity(1024)
                .build(),
            next_socket_id: AtomicU64::new(1),
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

    /// Tells one sender their line did not go through.
    ///
    /// Addressed to that connection and never fanned out: a refusal is between the server and
    /// the person who typed it, and the rest of the world never saw the message to begin with.
    pub fn reject(&self, author: &str, rejection: &ChatRejection, text: &str) {
        tracing::info!(player = %author, rejection = %rejection, "chat send refused");
        let packet = ChatRejectedPacket::new(rejection.to_string(), text.to_string());
        if let Ok(sinks) = self.sinks.read() {
            for sink in sinks.iter() {
                sink.deliver_rejection(author, &packet);
            }
        }
    }

    /// Claims an identity for one connection, before it registers.
    ///
    /// Held by the caller for the life of the connection and handed back at teardown, so a
    /// socket only ever releases the registration it still owns.
    pub fn next_socket_id(&self) -> u64 {
        self.next_socket_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn register(
        &self,
        id: u64,
        world_uuid: String,
        world_name: String,
        tx: mpsc::Sender<String>,
    ) -> Option<mpsc::Sender<String>> {
        tracing::info!(world = %world_uuid, name = %world_name, socket = id, "chat channel registered");
        self.registry.register(id, world_uuid, world_name, tx)
    }

    /// Registers one socket under every id its room spans.
    pub fn register_room(
        &self,
        id: u64,
        worlds: &[String],
        world_name: String,
        tx: mpsc::Sender<String>,
    ) -> Vec<mpsc::Sender<String>> {
        tracing::info!(
            worlds = ?worlds,
            name = %world_name,
            socket = id,
            "chat channel registered"
        );
        self.registry.register_room(id, worlds, world_name, tx)
    }

    pub fn unregister(&self, world_uuid: &str, id: u64) {
        tracing::info!(world = %world_uuid, socket = id, "chat channel unregistered");
        self.registry.unregister(world_uuid, id);
    }

    pub fn is_available(&self, world_uuid: &str) -> bool {
        self.registry.contains(world_uuid)
    }

    pub fn world_name(&self, world_uuid: &str) -> Option<String> {
        self.registry.world_name(world_uuid)
    }

    /// Every room currently relaying chat, as (canonical id, world name).
    ///
    /// A world is reachable as soon as its mod connects, which is well before anybody joins.
    /// Listing only worlds the player has been seen in left the composer dead on an empty
    /// server — the mod was connected and willing, and the app had no way to know.
    pub fn rooms(&self) -> Vec<(String, String)> {
        self.registry.rooms()
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

        for world_uuid in worlds {
            let packet = ChatMessagePacket::new(
                Some(author.clone()),
                text.clone(),
                Some(world_uuid.clone()),
                ChatOrigin::Game,
            );
            self.fan_out(world_uuid, None, &packet);
        }

        self.spawn_history(canonical, &author);
    }

    /// Something the server said: a death, a join, a leave, a broadcast.
    ///
    /// No author, so it renders as a system line — quieter, and unmistakable for a person
    /// talking. No history is recorded: nobody was speaking.
    pub async fn on_game_event(&self, worlds: &[String], text: String) {
        for world_uuid in worlds {
            let packet = ChatMessagePacket::new(
                None,
                text.clone(),
                Some(world_uuid.clone()),
                ChatOrigin::Game,
            );
            self.fan_out(world_uuid, None, &packet);
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
        // Every refusal below is answered to the sender before it is returned. A rejection only
        // the server knows about is indistinguishable, from the composer, from a message that
        // landed — which is the whole reason a typed line could disappear.
        let refuse = |rejection: ChatRejection| -> ChatRejection {
            self.reject(author, &rejection, &text);
            rejection
        };

        // Resolve to the room rather than the id. On Paper and Fabric the id names a
        // dimension, and the room is all of them: a player standing in the nether is in the
        // same room as the overworld id the app addressed.
        let Some(room) = self.registry.room(world_uuid) else {
            return Err(refuse(ChatRejection::NoChannel));
        };

        // The sender's live world beats the world they named — they may have been transferred
        // while the app still held the older target. Compared against the room, so changing
        // dimension is not mistaken for changing server.
        if let Some(current) = self.current_world_of(author).await {
            if !room.contains(&current) {
                return Err(refuse(ChatRejection::WrongWorld {
                    current: Some(current),
                }));
            }
        }

        let Some(tx) = self.registry.sender(world_uuid) else {
            return Err(refuse(ChatRejection::NoChannel));
        };

        // The gamertag, not the certificate CN. `minecraft:Alaydriem` is how the voice plane
        // keys identity; it is not what belongs in a chat line.
        let display = Self::display_name(author);

        let frame = ChatFrame::Say {
            author: display.clone(),
            text: text.clone(),
        };
        let Ok(body) = serde_json::to_string(&frame) else {
            return Err(refuse(ChatRejection::NoChannel));
        };
        if tx.try_send(body).is_err() {
            return Err(refuse(ChatRejection::NoChannel));
        }

        // Delivered to the whole room, exactly as a typed line is. Delivering only under the
        // id the client named would hide it from anyone in another dimension.
        //
        // The author is named on the first id only. A sink guarantees their copy from that,
        // and naming them per id would echo the line once per dimension in the room.
        for (index, id) in room.iter().enumerate() {
            let packet = ChatMessagePacket::new(
                Some(display.clone()),
                text.clone(),
                Some(id.clone()),
                ChatOrigin::App,
            );
            self.fan_out(id, (index == 0).then_some(author), &packet);
        }
        Ok(())
    }

    fn fan_out(&self, world_uuid: &str, author_identity: Option<&str>, packet: &ChatMessagePacket) {
        if let Ok(sinks) = self.sinks.read() {
            for sink in sinks.iter() {
                sink.deliver(world_uuid, author_identity, packet);
            }
        }
    }

    /// The world this identity is standing in right now, if any.
    ///
    /// The world list cannot come from history alone: history is written when somebody speaks,
    /// so a player who has never typed in a world would never see it offered — and could
    /// therefore never type there. Presence breaks that deadlock.
    pub async fn live_world_of(&self, identity: &str) -> Option<String> {
        self.current_world_of(identity).await
    }

    /// Records that this player is in this world, so the picker can offer it after they leave.
    ///
    /// Throttled: positions arrive at 4 Hz and the row only needs to be fresh enough to order
    /// a list.
    pub async fn note_presence(&self, identity: &str, world_uuid: &str) {
        let key = (identity.to_string(), world_uuid.to_string());
        if self.recently_recorded.get(&key).await.is_some() {
            return;
        }
        self.recently_recorded.insert(key, ()).await;

        let author = Self::display_name(identity);
        self.record_history(world_uuid, &author).await;
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

        let name = self
            .world_name(world_uuid)
            .unwrap_or_else(|| world_uuid.to_string());

        Self::write_history(db, identities, world_uuid, name, author).await;
    }

    /// The same row, written without the caller waiting for it.
    ///
    /// Two database round trips sit behind it. A mod on the embedded server reaches this from
    /// the game's own thread over the FFI, holding the lock the position tick also takes, so a
    /// chat line that waited on storage would stall the server the line came from. Nothing
    /// downstream reads the row within the lifetime of the message, and the rows are an upsert
    /// of a timestamp, so they are order-insensitive.
    fn spawn_history(&self, world_uuid: &str, author: &str) {
        let (Some(db), Some(identities)) = (self.db.get(), self.identities.get()) else {
            return;
        };

        let db = db.clone();
        let identities = identities.clone();
        let world_uuid = world_uuid.to_string();
        let author = author.to_string();
        let name = self
            .world_name(&world_uuid)
            .unwrap_or_else(|| world_uuid.clone());

        tokio::spawn(async move {
            Self::write_history(&db, &identities, &world_uuid, name, &author).await;
        });
    }

    async fn write_history(
        db: &DatabaseConnection,
        identities: &PlayerIdentityService,
        world_uuid: &str,
        world_name: String,
        author: &str,
    ) {
        let Some(player_id) = identities
            .find_player_id_by_gamertag(author, &Game::Minecraft)
            .await
        else {
            return;
        };

        let now = common::ncryptflib::rocket::Utc::now().timestamp();

        let row = entity::player_world::ActiveModel {
            player_id: ActiveValue::Set(player_id),
            world_uuid: ActiveValue::Set(world_uuid.to_string()),
            world_name: ActiveValue::Set(world_name),
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
            .exec(db)
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
