use crate::stream::quic::connection_registry::ConnectionRegistry;
use anyhow::Error;
use common::structs::channel::{ChannelCollection, ChannelEvents};
use common::structs::packet::{
    ChannelEventPacket, PacketType, PlayerDataPacket, QuicNetworkPacket,
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
        }
    }

    pub fn set_connection_registry(&mut self, registry: Arc<ConnectionRegistry>) {
        self.connection_registry = Some(registry);
    }

    pub fn get_player_cache(&self) -> Arc<Cache<String, PlayerEnum>> {
        self.player_cache.clone()
    }

    pub fn get_channel_collection(&self) -> Arc<ChannelCollection> {
        self.channel_collection.clone()
    }

    pub async fn process_packet(&self, packet: QuicNetworkPacket) -> Result<(), Error> {
        match packet.packet_type {
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

    pub async fn remove_player(&self, player_name: &str) -> Result<Vec<String>, Error> {
        self.player_cache.remove(player_name).await;

        if let Some(registry) = &self.connection_registry {
            registry.remove_player_channel(player_name);
        }

        let removed_channels = self.channel_collection
            .remove_player_from_all_channels(player_name)
            .await;

        tracing::debug!(
            "Removed player {} from caches on disconnect (was in {} channels)",
            player_name,
            removed_channels.len()
        );
        Ok(removed_channels)
    }
}
