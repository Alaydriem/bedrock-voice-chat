use anyhow::Error;
use common::structs::channel::ChannelEvents;
use common::structs::packet::{
    ChannelEventPacket, PacketType, PlayerDataPacket, QuicNetworkPacket,
};
use common::Player;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// Manages player position cache and channel membership cache
#[derive(Clone)]
pub struct CacheManager {
    // Player position cache - expires after 5 minutes
    player_cache: Arc<Cache<String, Player>>,
    // Player to channel mapping cache
    channel_cache: Arc<Cache<String, String>>,
}

impl CacheManager {
    pub fn new() -> Self {
        let player_cache = Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(300)) // 5 minutes
                .max_capacity(256)
                .build(),
        );

        let channel_cache = Arc::new(Cache::builder().max_capacity(100).build());

        Self {
            player_cache,
            channel_cache,
        }
    }

    pub fn get_player_cache(&self) -> Arc<Cache<String, Player>> {
        self.player_cache.clone()
    }

    pub fn get_channel_cache(&self) -> Arc<Cache<String, String>> {
        self.channel_cache.clone()
    }

    /// Process packets and update caches accordingly
    pub async fn process_packet(&self, packet: QuicNetworkPacket) -> Result<(), Error> {
        match packet.packet_type {
            PacketType::PlayerData => {
                if let Some(data) = packet.get_data() {
                    let data: Result<PlayerDataPacket, ()> = data.to_owned().try_into();
                    if let Ok(player_data) = data {
                        for player in player_data.players {
                            self.player_cache
                                .insert(player.name.clone(), player.clone())
                                .await;
                            tracing::debug!("Updated player position cache for: {}", player.name);
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
                                self.channel_cache
                                    .insert(channel_data.name.clone(), channel_data.channel.clone())
                                    .await;
                                tracing::info!(
                                    "Player {} joined channel {}",
                                    channel_data.name,
                                    channel_data.channel
                                );
                            }
                            ChannelEvents::Leave => {
                                self.channel_cache.remove(&channel_data.name).await;
                                tracing::info!("Player {} left channel", channel_data.name);
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
            // Use the existing update_coordinates method from the packet itself
            packet.update_coordinates(self.player_cache.clone()).await;
            tracing::debug!(
                "Updated coordinates for AudioFrame packet from player: {}",
                packet.get_author()
            );
        }
        Ok(packet)
    }

    /// Remove a player from the cache when they disconnect
    /// This is called when a player disconnects to clean up cache entries
    pub async fn remove_player(&self, player_name: &str) -> Result<(), Error> {
        // Remove from player position cache
        self.player_cache.remove(player_name).await;

        // Remove from channel membership cache
        self.channel_cache.remove(player_name).await;

        tracing::info!("Removed player {} from caches on disconnect", player_name);
        Ok(())
    }
}
