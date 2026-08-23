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

use common::curia;
use crate::services::{BedrockEventService, ClientActionService};
use crate::stream::quic::connection::ConnectionRegistry;
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
    // Read handle on the live playbacks' speakers, so a server-injected sender resolves from
    // the store that owns its lifetime rather than from the position cache, whose TTL is a
    // presence lifetime and would lapse part-way through a track.
    //
    // `Arc<OnceLock<..>>` rather than an `Option`, because this type is `Clone` and the audio
    // path holds a clone made before the playback service exists. A plain `Option` would be set
    // on one copy and read as absent by the other.
    injected_speakers:
        Arc<std::sync::OnceLock<Arc<moka::future::Cache<String, crate::services::SpeakerEntry>>>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            players: PlayerCache::new(),
            channel_collection: Arc::new(ChannelCollection::new(100)),
            connection_registry: None,
            injected_speakers: Arc::new(std::sync::OnceLock::new()),
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

    /// Live relay worlds and how many players are in each, sorted by world name.
    ///
    /// Delegated rather than reached through `players()` so a caller outside this
    /// module never has to name the inner cache: the manager is the surface, and
    /// which cache answers is its business.
    pub fn relay_world_populations(&self) -> Vec<(String, usize)> {
        self.players.relay_world_populations()
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

    /// Wires the playback service's speaker registry in.
    ///
    /// Takes the service rather than its cache so callers never name the entry type, which is
    /// the playback service's own business.
    /// Set once; a later install is ignored, matching how the registry installs its metrics.
    pub fn set_injected_speakers(&self, playback: &crate::services::AudioPlaybackService) {
        let _ = self.injected_speakers.set(playback.speakers());
    }

    /// The player behind this packet's sender, whoever that is.
    ///
    /// Two stores, because a player and a server-injected speaker have different lifetimes: a
    /// player ages out of the position cache when they stop reporting, a playback's speaker
    /// expires with its track. Selected on the sender's shape rather than by trying both, so a
    /// player frame pays one lookup and neither store can answer for the other.
    ///
    /// `None` means nothing on this server knows where that sender is, and audio from it is not
    /// routable.
    pub async fn resolve_speaker(
        &self,
        packet: &QuicNetworkPacket,
    ) -> Option<common::PlayerEnum> {
        let key = packet.sender_key()?;

        match packet.sender_service() {
            Some(_) => self
                .injected_speakers
                .get()?
                .get(&key)
                .await
                .map(|entry| entry.player),
            None => self.players.inner_arc().get(&key).await,
        }
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
                                curia::warn!(
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
                    let sender = packet.sender_identity().map(|i| i.to_string());
                    curia::warn!("Dropping PlayerData received from a player connection", { "sender": sender.as_deref().unwrap_or("unknown") });
                    return Ok(());
                }

                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerDataPacket, ()> = data.to_owned().try_into();
                    if let Ok(player_data) = data {
                        for player in player_data.players {
                            use common::traits::player_data::PlayerData;
                            let identity = player.identity().to_string();
                            self.players.set(identity.clone(), player.clone()).await;
                            curia::debug!("Updated player position cache for: {}", identity);
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
                    let sender = packet.sender_identity().map(|i| i.to_string());
                    curia::warn!("Dropping ChannelEvent received from a player connection", { "sender": sender.as_deref().unwrap_or("unknown") });
                    return Ok(());
                }

                if let Some(data) = packet.get_data() {
                    let data: Result<ChannelEventPacket, ()> = data.to_owned().try_into();
                    if let Ok(channel_data) = data {
                        curia::info!(
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
                                        &channel_data.name.to_string(),
                                        &channel_data.channel,
                                    );
                                }

                                curia::info!(
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
                                    registry.remove_player_channel(&channel_data.name.to_string());
                                }

                                curia::info!(
                                    "Player {} left channel {}",
                                    channel_data.name,
                                    channel_data.channel
                                );
                            }
                            ChannelEvents::Create => {
                                curia::info!(
                                    "Channel {} created by {}",
                                    channel_data.channel,
                                    channel_data
                                        .creator
                                        .as_ref()
                                        .map(|c| c.to_string())
                                        .unwrap_or_else(|| "unknown".to_string())
                                );
                            }
                            ChannelEvents::Delete => {
                                self.channel_collection.remove(&channel_data.channel).await;

                                if let Some(registry) = &self.connection_registry {
                                    registry.remove_channel(&channel_data.channel);
                                }

                                curia::info!("Channel {} deleted", channel_data.channel);
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
                        curia::warn!("Received ChatSend but no ChatService is wired up");
                        return Ok(());
                    }
                };

                // Stamped from the certificate at ingress. An unstamped packet was injected by
                // this server rather than sent by a player, and attributing it to anyone would
                // let a client post as somebody else.
                let Some(author) = packet.sender_identity().map(|s| s.to_string()) else {
                    curia::warn!("Refusing an unattributed ChatSend");
                    return Ok(());
                };

                if let Some(data) = packet.get_data() {
                    let send: Result<ChatSendPacket, ()> = data.to_owned().try_into();
                    if let Ok(send) = send {
                        let Some(world) = send.world_uuid.clone() else {
                            // The composer sends what it has rather than deciding on the
                            // server's behalf, so an unnamed world is answered here instead of
                            // being dropped where the sender cannot see it.
                            service.reject(
                                &author,
                                &common::errors::ChatRejection::NoWorld,
                                &send.text,
                            );
                            return Ok(());
                        };
                        // `on_app_send` answers the sender itself, so nothing more is owed here.
                        let _ = service.on_app_send(&author, &world, send.text).await;
                    }
                }
            }
            PacketType::BedrockEvent => {
                let service = match &self.bedrock_event_service {
                    Some(s) => s.clone(),
                    None => {
                        curia::warn!(
                            "Received BedrockEvent packet but no BedrockEventService is wired up"
                        );
                        return Ok(());
                    }
                };

                let authenticated_player = packet
                    .sender_identity()
                    .map(|identity| identity.to_string())
                    .unwrap_or_default();
                if let Some(data) = packet.get_data() {
                    let event: Result<BedrockEventPacket, ()> = data.to_owned().try_into();
                    if let Ok(event) = event {
                        if let Err(rejection) = service
                            .handle_event(event, authenticated_player.clone())
                            .await
                        {
                            curia::warn!("BedrockEvent rejected", { "player": authenticated_player.to_string(), "rejection": rejection.to_string() });
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
                        let author = packet
                            .sender_identity()
                            .map(|identity| identity.to_string())
                            .unwrap_or_default();
                        if qs.state.id == author {
                            self.player_state.set(qs.state.id.clone(), qs.state).await;
                        } else {
                            curia::warn!(
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
                        let author = packet
                            .sender_identity()
                            .map(|identity| identity.to_string())
                            .unwrap_or_default();
                        if pp.preference.owner == author {
                            let key = PreferenceKey::new(
                                pp.preference.owner.clone(),
                                pp.preference.target.clone(),
                            );
                            self.preferences.set(key, pp.preference).await;
                        } else {
                            curia::warn!(
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
                let author = packet
                    .sender_identity()
                    .map(|identity| identity.to_string())
                    .unwrap_or_default();
                if !ca.action.action.is_group_action() {
                    // Self/preference actions apply on the actor's own client
                    // (the no-net proxy shortcut); nothing to do server-side.
                    curia::debug!(format!("Ignoring serverbound non-group ClientAction from {author}"));
                    return Ok(());
                }
                let Some(webhook) = &self.webhook_receiver else {
                    curia::warn!(
                        "Received serverbound group ClientAction but no webhook receiver is wired up"
                    );
                    return Ok(());
                };
                match ClientActionService::route_group(
                    &ca.action.action,
                    &ca.action.actor_identity(),
                    &self.channel_collection,
                    webhook,
                )
                .await
                {
                    Ok(created) => {
                        if let Some(code) = created {
                            curia::info!(format!("Group {code} created via QUIC control by {author}"));
                        }
                    }
                    // route_group only errors on a JoinGroup miss (unknown share
                    // code) — a client mistake, not a server fault.
                    Err(e) => curia::info!(format!("route_group (QUIC) rejected for {author}: {e}")),
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Attaches the speaker's position to a frame that will carry it.
    ///
    /// Skipped between heartbeats: `route_audio_frame` strips the speaker from those frames and
    /// resolves proximity from the caller's resolved player, so filling it here would be work
    /// nothing reads.
    ///
    /// Queries the attach interval without spending it. Consuming it here would leave the egress
    /// with nothing to attach, and no listener would ever receive a position.
    pub fn attach_speaker(
        &self,
        packet: &mut QuicNetworkPacket,
        speaker: Option<&common::PlayerEnum>,
    ) {
        let Some(speaker) = speaker else {
            return;
        };
        let Some(registry) = &self.connection_registry else {
            return;
        };
        // Read before `data` is borrowed mutably below.
        let Some(key) = packet.sender_key() else {
            return;
        };
        if !registry.sender_attach_pending(&key, std::time::Instant::now()) {
            return;
        }

        if let common::structs::packet::QuicNetworkPacketData::AudioFrame(ref mut audio) =
            packet.data
        {
            audio.speaker = Some(common::structs::packet::SpeakerPosition::from_player(speaker));
        }
    }

    /// Evicts every cache entry a disconnecting player owns.
    ///
    /// Returns each channel the player left with that channel's owner, because the caller
    /// fans a `Leave` per channel and every channel event names its owner.
    pub async fn remove_player(
        &self,
        identity: &common::PlayerIdentity,
    ) -> Result<Vec<(String, common::PlayerIdentity)>, Error> {
        let key = identity.to_string();
        self.players.delete(&key).await;

        // Evict this player's control-plane state so a disconnected player's mute/
        // record status and per-player prefs stop being served.
        self.player_state.delete(&key).await;
        self.preferences.evict_owner(&key).await;

        if let Some(registry) = &self.connection_registry {
            registry.remove_player_channel(&key);
        }

        let removed_channels = self
            .channel_collection
            .remove_player_from_all_channels(identity)
            .await;

        curia::debug!(
            "Removed player {} from caches on disconnect (was in {} channels)",
            identity,
            removed_channels.len()
        );
        Ok(removed_channels)
    }
}
