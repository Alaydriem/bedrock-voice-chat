use anyhow::Error;
use common::structs::channel::{ChannelCollection, ChannelEvents};
use common::structs::packet::{
    ChannelEventPacket, PacketType, PlayerDataPacket, QuicNetworkPacket,
};
use common::PlayerEnum;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// Manages player position cache and channel collection.
///
/// The ChannelCollection is the single source of truth for channel membership.
#[derive(Clone)]
pub struct CacheManager {
    /// Player position cache data
    player_cache: Arc<Cache<String, PlayerEnum>>,
    /// Channel collection managing channel memberships
    channel_collection: Arc<ChannelCollection>,
}

impl CacheManager {
    pub fn new() -> Self {
        let player_cache = Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(300)) // 5 minutes
                .max_capacity(256)
                .build(),
        );

        let channel_collection = Arc::new(ChannelCollection::new(100));

        Self {
            player_cache,
            channel_collection,
        }
    }

    pub fn get_player_cache(&self) -> Arc<Cache<String, PlayerEnum>> {
        self.player_cache.clone()
    }

    pub fn get_channel_collection(&self) -> Arc<ChannelCollection> {
        self.channel_collection.clone()
    }

    /// Process packets and update caches accordingly
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
                                    channel_data
                                        .creator
                                        .as_deref()
                                        .unwrap_or("unknown")
                                );
                            }
                            ChannelEvents::Delete => {
                                self.channel_collection
                                    .remove(&channel_data.channel)
                                    .await;

                                tracing::info!(
                                    "Channel {} deleted",
                                    channel_data.channel
                                );
                            }
                        }
                    }
                }
            }
            _ => {
                // Other packet types don't need caching
            }
        }
        Ok(())
    }

    /// Update coordinates for AudioFrame packets and return the updated packet
    /// This is called for each AudioFrame to update player positions before broadcasting
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

    /// Remove a player from all caches when they disconnect or reconnect.
    /// Returns the list of channel IDs the player was removed from.
    pub async fn remove_player(&self, player_name: &str) -> Result<Vec<String>, Error> {
        self.player_cache.remove(player_name).await;

        let removed_channels = self
            .channel_collection
            .remove_player_from_all_channels(player_name)
            .await;

        tracing::debug!(
            "Removed player {} from caches (was in {} channels)",
            player_name,
            removed_channels.len()
        );
        Ok(removed_channels)
    }
}
