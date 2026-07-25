mod cache_trait;
mod player_cache;
mod player_preference_cache;
mod player_state_cache;

pub use cache_trait::CacheTrait;
pub use player_cache::PlayerCache;
pub use player_preference_cache::PlayerPreferenceCache;
pub use player_state_cache::PlayerStateCache;

use crate::services::{BedrockEventService, ClientActionService};
use crate::stream::quic::connection_registry::ConnectionRegistry;
use crate::stream::quic::webhook_receiver::WebhookReceiver;
use anyhow::Error;
use common::Game;
use common::structs::channel::{ChannelCollection, ChannelEvents};
use common::structs::control::PreferenceKey;
use common::structs::packet::{
    BedrockEventPacket, ChannelEventPacket, ClientActionPacket, PacketDirection, PacketType,
    PlayerDataPacket, PlayerPositionPacket, PlayerPreferencePacket, QueryStatePacket,
    QuicNetworkPacket,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct CacheManager {
    players: PlayerCache,
    channel_collection: Arc<ChannelCollection>,
    connection_registry: Option<Arc<ConnectionRegistry>>,
    bedrock_event_service: Option<Arc<BedrockEventService>>,
    // Fan-out sender for group ClientActions arriving ServerBound over QUIC
    // (the no-net path); the HTTP control route receives its own via State.
    webhook_receiver: Option<WebhookReceiver>,
    player_state: PlayerStateCache,
    preferences: PlayerPreferenceCache,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            players: PlayerCache::new(),
            channel_collection: Arc::new(ChannelCollection::new(100)),
            connection_registry: None,
            bedrock_event_service: None,
            webhook_receiver: None,
            player_state: PlayerStateCache::new(),
            preferences: PlayerPreferenceCache::new(),
        }
    }

    /// The position/identity cache.
    pub fn players(&self) -> &PlayerCache {
        &self.players
    }

    /// The player self-state cache (`get`/`set`/`delete` via `CacheTrait`).
    pub fn player_state(&self) -> &PlayerStateCache {
        &self.player_state
    }

    /// The per-player preference cache (`CacheTrait` + `get_scoped`/`evict_owner`).
    pub fn preferences(&self) -> &PlayerPreferenceCache {
        &self.preferences
    }

    pub(crate) fn set_connection_registry(&mut self, registry: Arc<ConnectionRegistry>) {
        self.connection_registry = Some(registry);
    }

    pub fn set_bedrock_event_service(&mut self, service: Arc<BedrockEventService>) {
        self.bedrock_event_service = Some(service);
    }

    pub fn set_webhook_receiver(&mut self, webhook: WebhookReceiver) {
        self.webhook_receiver = Some(webhook);
    }

    pub fn get_channel_collection(&self) -> Arc<ChannelCollection> {
        self.channel_collection.clone()
    }

    pub fn get_connection_registry(&self) -> Option<Arc<ConnectionRegistry>> {
        self.connection_registry.clone()
    }

    // `authenticated_game` is the game from the sender's mTLS certificate CN, or
    // `None` for server-injected packets that arrive without a certificate (the
    // webhook path). It is only consulted where a membership key must be built.
    pub async fn process_packet(
        &self,
        packet: QuicNetworkPacket,
        authenticated_game: Option<common::Game>,
    ) -> Result<(), Error> {
        match packet.packet_type {
            PacketType::PlayerPosition => {
                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerPositionPacket, ()> = data.to_owned().try_into();
                    if let Ok(pos) = data {
                        let author = packet.get_author();
                        if !author.is_empty() {
                            self.players.set(author, pos.player).await;
                        }
                    }
                }
            }
            PacketType::PlayerData => {
                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerDataPacket, ()> = data.to_owned().try_into();
                    if let Ok(player_data) = data {
                        for player in player_data.players {
                            use common::traits::player_data::PlayerData;
                            let player_name = player.get_name().to_string();
                            self.players
                                .set(player_name.clone(), player.clone())
                                .await;
                            tracing::debug!("Updated player position cache for: {}", player_name);
                        }
                    }
                }
            }
            PacketType::ChannelEvent => {
                if let Some(data) = packet.get_data() {
                    let data: Result<ChannelEventPacket, ()> = data.to_owned().try_into();
                    if let Ok(channel_data) = data {
                        tracing::info!(
                            "[{}] {:?} {}",
                            channel_data.name,
                            channel_data.event,
                            channel_data.channel
                        );

                        match channel_data.event {
                            ChannelEvents::Join => {
                                self.channel_collection
                                    .add_player_to_channel(
                                        &channel_data.name,
                                        &channel_data.channel,
                                    )
                                    .await;

                                if let Some(registry) = &self.connection_registry {
                                    registry.update_player_channel(
                                        channel_data.name.clone(),
                                        channel_data.channel.clone(),
                                    );
                                }

                                tracing::info!(
                                    "Player {} joined channel {}",
                                    channel_data.name,
                                    channel_data.channel
                                );
                            }
                            ChannelEvents::Leave => {
                                self.channel_collection
                                    .remove_player_from_channel(
                                        &channel_data.name,
                                        &channel_data.channel,
                                    )
                                    .await;

                                if let Some(registry) = &self.connection_registry {
                                    registry.remove_player_channel(&channel_data.name);
                                }

                                tracing::info!(
                                    "Player {} left channel {}",
                                    channel_data.name,
                                    channel_data.channel
                                );
                            }
                            ChannelEvents::Create => {
                                tracing::info!(
                                    "Channel {} created by {}",
                                    channel_data.channel,
                                    channel_data.creator.as_deref().unwrap_or("unknown")
                                );
                            }
                            ChannelEvents::Delete => {
                                self.channel_collection.remove(&channel_data.channel).await;

                                if let Some(registry) = &self.connection_registry {
                                    registry.remove_channel(&channel_data.channel);
                                }

                                tracing::info!("Channel {} deleted", channel_data.channel);
                            }
                            ChannelEvents::Rename => {}
                        }
                    }
                }
            }
            PacketType::BedrockEvent => {
                let service = match &self.bedrock_event_service {
                    Some(s) => s.clone(),
                    None => {
                        tracing::warn!(
                            "Received BedrockEvent packet but no BedrockEventService is wired up"
                        );
                        return Ok(());
                    }
                };

                let authenticated_player = packet.get_author();
                if let Some(data) = packet.get_data() {
                    let event: Result<BedrockEventPacket, ()> = data.to_owned().try_into();
                    if let Ok(event) = event {
                        if let Err(rejection) = service
                            .handle_event(event, authenticated_player.clone())
                            .await
                        {
                            tracing::warn!(
                                player = %authenticated_player,
                                rejection = %rejection,
                                "BedrockEvent rejected"
                            );
                        }
                    }
                }
            }
            PacketType::QueryState => {
                if let Some(data) = packet.get_data() {
                    let data: Result<QueryStatePacket, ()> = data.to_owned().try_into();
                    if let Ok(qs) = data {
                        // A client may only report its OWN state; anchor to the
                        // connection identity so it can't poison another player's.
                        let author = packet.get_author();
                        if qs.state.id == author {
                            self.player_state.set(qs.state.id.clone(), qs.state).await;
                        } else {
                            tracing::warn!(
                                "Dropping QueryState: id {} != author {}",
                                qs.state.id,
                                author
                            );
                        }
                    }
                }
            }
            PacketType::PlayerPreference => {
                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerPreferencePacket, ()> = data.to_owned().try_into();
                    if let Ok(pp) = data {
                        let author = packet.get_author();
                        if pp.preference.owner == author {
                            let key = PreferenceKey::new(
                                pp.preference.owner.clone(),
                                pp.preference.target.clone(),
                            );
                            self.preferences.set(key, pp.preference).await;
                        } else {
                            tracing::warn!(
                                "Dropping PlayerPreference: owner {} != author {}",
                                pp.preference.owner,
                                author
                            );
                        }
                    }
                }
            }
            PacketType::ClientAction => {
                let Some(data) = packet.get_data() else {
                    return Ok(());
                };
                let Ok(ca): Result<ClientActionPacket, ()> = data.to_owned().try_into() else {
                    return Ok(());
                };
                // ClientBound copies are routed to clients, never consumed here.
                if ca.direction != PacketDirection::ServerBound {
                    return Ok(());
                }
                // The wire id is untrusted: anchor the actor to the connection
                // author, the same guard QueryState/PlayerPreference use.
                let author = packet.get_author();
                if !ca.action.action.is_group_action() {
                    // Self/preference actions apply on the actor's own client
                    // (the no-net proxy shortcut); nothing to do server-side.
                    tracing::debug!(
                        "Ignoring serverbound non-group ClientAction from {author}"
                    );
                    return Ok(());
                }
                let Some(webhook) = &self.webhook_receiver else {
                    tracing::warn!(
                        "Received serverbound group ClientAction but no webhook receiver is wired up"
                    );
                    return Ok(());
                };
                // The game comes from the authenticated certificate, so a Hytale
                // actor is keyed as `hytale:name` rather than being assumed to be
                // Minecraft. Falls back to Minecraft only for callers with no
                // certificate context.
                let actor_cn = authenticated_game
                    .unwrap_or(Game::Minecraft)
                    .membership_key(&author);
                match ClientActionService::new()
                    .route_group(&ca.action.action, &actor_cn, &self.channel_collection, webhook)
                    .await
                {
                    Ok(created) => {
                        if let Some(code) = created {
                            tracing::info!("Group {code} created via QUIC control by {author}");
                        }
                    }
                    // route_group only errors on a JoinGroup miss (unknown share
                    // code) — a client mistake, not a server fault.
                    Err(e) => tracing::info!("route_group (QUIC) rejected for {author}: {e}"),
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn update_coordinates(
        &self,
        mut packet: QuicNetworkPacket,
    ) -> Result<QuicNetworkPacket, Error> {
        if packet.packet_type == PacketType::AudioFrame {
            packet.update_coordinates(self.players.inner_arc()).await;
            tracing::debug!(
                "Updated coordinates for AudioFrame packet from player: {}",
                packet.get_author()
            );
        }
        Ok(packet)
    }

    pub async fn remove_player(
        &self,
        player_name: &str,
        game: Option<common::Game>,
    ) -> Result<Vec<String>, Error> {
        use common::traits::player_data::PlayerData;

        // Channel membership is keyed by the cert common name (`game:gamertag`),
        // the same key the channel event handler and `route_audio_frame` use.
        // The bare gamertag never matches it, so resolve the canonical key from
        // the caller-supplied game — or, failing that, the cached player's game —
        // before evicting the entry.
        let resolved_game = match game {
            Some(g) => Some(g),
            None => self
                .players
                .get(&player_name.to_string())
                .await
                .map(|player| player.get_game()),
        };

        self.players.delete(&player_name.to_string()).await;

        // Evict this player's control-plane state so a disconnected player's mute/
        // record status and per-player prefs stop being served.
        self.player_state.delete(&player_name.to_string()).await;
        self.preferences.evict_owner(player_name).await;

        let membership_key = match &resolved_game {
            Some(g) => g.membership_key(player_name),
            None => player_name.to_string(),
        };

        if let Some(registry) = &self.connection_registry {
            registry.remove_player_channel(&membership_key);
        }

        let removed_channels = self
            .channel_collection
            .remove_player_from_all_channels(&membership_key)
            .await;

        tracing::debug!(
            "Removed player {} (membership key {}) from caches on disconnect (was in {} channels)",
            player_name,
            membership_key,
            removed_channels.len()
        );
        Ok(removed_channels)
    }
}
