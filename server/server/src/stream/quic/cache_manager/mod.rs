mod cache_trait;
mod player_cache;
mod player_preference_cache;
mod player_state_cache;
mod websocket_ticket_cache;

pub use cache_trait::CacheTrait;
pub use player_cache::PlayerCache;
pub use player_preference_cache::PlayerPreferenceCache;
pub use player_state_cache::PlayerStateCache;
pub use websocket_ticket_cache::{TicketIdentity, WebsocketTicketCache};

use crate::services::{BedrockEventService, ClientActionService};
use crate::stream::quic::connection_registry::ConnectionRegistry;
use crate::stream::quic::webhook_receiver::WebhookReceiver;
use anyhow::Error;
use common::structs::channel::{ChannelCollection, ChannelEvents};
use common::structs::control::PreferenceKey;
use common::structs::packet::{
    ChatSendPacket,
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
    chat_service: Option<Arc<crate::services::ChatService>>,
    // Fan-out sender for group ClientActions arriving ServerBound over QUIC
    // (the no-net path); the HTTP control route receives its own via State.
    webhook_receiver: Option<WebhookReceiver>,
    player_state: PlayerStateCache,
    preferences: PlayerPreferenceCache,
    websocket_tickets: WebsocketTicketCache,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            players: PlayerCache::new(),
            channel_collection: Arc::new(ChannelCollection::new(100)),
            connection_registry: None,
            bedrock_event_service: None,
            chat_service: None,
            webhook_receiver: None,
            player_state: PlayerStateCache::new(),
            preferences: PlayerPreferenceCache::new(),
            websocket_tickets: WebsocketTicketCache::new(),
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

    /// Single-use tickets exchanging an mTLS identity for a WebSocket upgrade.
    pub fn websocket_tickets(&self) -> &WebsocketTicketCache {
        &self.websocket_tickets
    }

    pub(crate) fn set_connection_registry(&mut self, registry: Arc<ConnectionRegistry>) {
        self.connection_registry = Some(registry);
    }

    pub fn set_chat_service(&mut self, service: Arc<crate::services::ChatService>) {
        self.chat_service = Some(service);
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

    // Every guard below anchors to `packet.sender_identity()`, which the QUIC ingress
    // stamped from the certificate. An unstamped packet was injected by this server rather
    // than sent by a player, and the guards refuse to attribute it to anyone.
    pub async fn process_packet(&self, packet: QuicNetworkPacket) -> Result<(), Error> {
        match packet.packet_type {
            PacketType::PlayerPosition => {
                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerPositionPacket, ()> = data.to_owned().try_into();
                    if let Ok(pos) = data {
                        // The stamped identity is authoritative; the name on `pos.player`
                        // is the client's own claim and is never the key.
                        //
                        // Only the client's Bedrock proxy emits this type, always over an
                        // authenticated connection. Unstamped there is no key any reader
                        // could resolve, so caching it would be write-only.
                        match packet.sender_identity() {
                            Some(identity) => {
                                self.players.set(identity.to_string(), pos.player).await;
                            }
                            None => {
                                tracing::warn!(
                                    "Dropping PlayerPosition with no authenticated sender"
                                );
                            }
                        }
                    }
                }
            }
            PacketType::PlayerData => {
                // Carries an entry per player, so its keys are the names inside the body
                // rather than the sender. That is only sound for something this server
                // injected: no client produces this type, and one arriving from a player
                // connection is a client writing arbitrary players' coordinates. Those
                // coordinates are what `route_audio_frame` resolves proximity from, so
                // accepting one would let a sender place itself beside anybody.
                if packet.sender_device().is_some() {
                    tracing::warn!(
                        sender = packet.sender_identity().unwrap_or("unknown"),
                        "Dropping PlayerData received from a player connection"
                    );
                    return Ok(());
                }

                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerDataPacket, ()> = data.to_owned().try_into();
                    if let Ok(player_data) = data {
                        for player in player_data.players {
                            use common::traits::player_data::PlayerData;
                            let identity = player.identity();
                            self.players.set(identity.clone(), player.clone()).await;
                            tracing::debug!("Updated player position cache for: {}", identity);
                        }
                    }
                }
            }
            PacketType::ChannelEvent => {
                // `channel_data.name` names the player the membership change applies to,
                // which is legitimately somebody other than the sender when the channel API
                // acts on a player's behalf. Nothing a client sends may say that: from a
                // player connection this type would join or remove any player from any
                // channel, and channel membership bypasses the proximity gate entirely.
                if packet.sender_device().is_some() {
                    tracing::warn!(
                        sender = packet.sender_identity().unwrap_or("unknown"),
                        "Dropping ChannelEvent received from a player connection"
                    );
                    return Ok(());
                }

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
            PacketType::ChatSend => {
                let service = match &self.chat_service {
                    Some(s) => s.clone(),
                    None => {
                        tracing::warn!("Received ChatSend but no ChatService is wired up");
                        return Ok(());
                    }
                };

                // Stamped from the certificate at ingress. An unstamped packet was injected by
                // this server rather than sent by a player, and attributing it to anyone would
                // let a client post as somebody else.
                let Some(author) = packet.sender_identity().map(|s| s.to_string()) else {
                    tracing::warn!("Refusing an unattributed ChatSend");
                    return Ok(());
                };

                if let Some(data) = packet.get_data() {
                    let send: Result<ChatSendPacket, ()> = data.to_owned().try_into();
                    if let Ok(send) = send {
                        let Some(world) = send.world_uuid.clone() else {
                            tracing::debug!(player = %author, "ChatSend named no world");
                            return Ok(());
                        };
                        if let Err(rejection) =
                            service.on_app_send(&author, &world, send.text).await
                        {
                            tracing::info!(
                                player = %author,
                                world = %world,
                                rejection = %rejection,
                                "ChatSend rejected"
                            );
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

                let authenticated_player = packet.sender_identity().unwrap_or_default().to_string();
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
                        let author = packet.sender_identity().unwrap_or_default();
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
                        let author = packet.sender_identity().unwrap_or_default();
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
                let author = packet.sender_identity().unwrap_or_default();
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
                match ClientActionService::route_group(
                    &ca.action.action,
                    author,
                    &self.channel_collection,
                    webhook,
                )
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
                "Updated coordinates for AudioFrame packet from player: {:?}",
                packet.sender_identity()
            );
        }
        Ok(packet)
    }

    /// Evicts every cache entry a disconnecting player owns.
    ///
    /// `identity` is the canonical `game:gamertag`, because that is the key all five caches
    /// share. A caller holding the game and the bare name loose composes it with
    /// `Game::membership_key` first — passing a bare name here silently matches nothing.
    pub async fn remove_player(&self, identity: &str) -> Result<Vec<String>, Error> {
        self.players.delete(&identity.to_string()).await;

        // Evict this player's control-plane state so a disconnected player's mute/
        // record status and per-player prefs stop being served.
        self.player_state.delete(&identity.to_string()).await;
        self.preferences.evict_owner(identity).await;

        if let Some(registry) = &self.connection_registry {
            registry.remove_player_channel(identity);
        }

        let removed_channels = self
            .channel_collection
            .remove_player_from_all_channels(identity)
            .await;

        tracing::debug!(
            "Removed player {} from caches on disconnect (was in {} channels)",
            identity,
            removed_channels.len()
        );
        Ok(removed_channels)
    }
}
