use crate::services::BedrockEventService;
use crate::stream::quic::connection_registry::ConnectionRegistry;
use anyhow::Error;
use common::structs::channel::{ChannelCollection, ChannelEvents};
use common::structs::packet::{
    BedrockEventPacket, ChannelEventPacket, PacketType, PlayerDataPacket, PlayerPositionPacket,
    QuicNetworkPacket,
};
use common::PlayerEnum;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct CacheManager {
    player_cache: Arc<Cache<String, PlayerEnum>>,
    channel_collection: Arc<ChannelCollection>,
    connection_registry: Option<Arc<ConnectionRegistry>>,
    bedrock_event_service: Option<Arc<BedrockEventService>>,
}

impl CacheManager {
    pub fn new() -> Self {
        let player_cache = Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .max_capacity(256)
                .build(),
        );

        let channel_collection = Arc::new(ChannelCollection::new(100));

        Self {
            player_cache,
            channel_collection,
            connection_registry: None,
            bedrock_event_service: None,
        }
    }

    pub(crate) fn set_connection_registry(&mut self, registry: Arc<ConnectionRegistry>) {
        self.connection_registry = Some(registry);
    }

    pub fn set_bedrock_event_service(&mut self, service: Arc<BedrockEventService>) {
        self.bedrock_event_service = Some(service);
    }

    pub fn get_player_cache(&self) -> Arc<Cache<String, PlayerEnum>> {
        self.player_cache.clone()
    }

    // Distinct `relay_world_uuid`s of players currently in the routing cache.
    // Backs the relay `ActiveWorldsSource` so the background register/lookup
    // task advertises only the worlds this server is actively hosting clients
    // in. Lock-light (a snapshot iteration over the moka cache); never on the
    // audio hot path.
    pub fn active_relay_worlds(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut seen: HashSet<String> = HashSet::new();
        for (_, player) in self.player_cache.iter() {
            if let Some(mc) = player.as_minecraft() {
                if let Some(world) = &mc.relay_world_uuid {
                    seen.insert(world.clone());
                }
            }
        }
        seen.into_iter().collect()
    }

    pub fn get_channel_collection(&self) -> Arc<ChannelCollection> {
        self.channel_collection.clone()
    }

    pub async fn process_packet(&self, packet: QuicNetworkPacket) -> Result<(), Error> {
        match packet.packet_type {
            PacketType::PlayerPosition => {
                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerPositionPacket, ()> = data.to_owned().try_into();
                    if let Ok(pos) = data {
                        let author = packet.get_author();
                        if !author.is_empty() {
                            self.player_cache.insert(author, pos.player).await;
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
                            self.player_cache
                                .insert(player_name.clone(), player.clone())
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
                                self.channel_collection
                                    .remove(&channel_data.channel)
                                    .await;

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
            _ => {}
        }
        Ok(())
    }

    pub async fn update_coordinates(
        &self,
        mut packet: QuicNetworkPacket,
    ) -> Result<QuicNetworkPacket, Error> {
        if packet.packet_type == PacketType::AudioFrame {
            packet.update_coordinates(self.player_cache.clone()).await;
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
                .player_cache
                .get(player_name)
                .await
                .map(|player| player.get_game()),
        };

        self.player_cache.remove(player_name).await;

        let membership_key = match &resolved_game {
            Some(g) => format!("{}:{}", g.as_str(), player_name),
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
