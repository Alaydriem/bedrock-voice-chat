use anyhow::Error;
use common::structs::channel::{Channel, ChannelEvents};
use common::structs::channel_player::ChannelPlayer;
use common::structs::packet::{
    ChannelEventPacket, PacketType, PlayerDataPacket, QuicNetworkPacket,
};
use common::PlayerEnum;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// Manages player position cache and channel cache
#[derive(Clone)]
pub struct CacheManager {
    /// Player position cache data
    player_cache: Arc<Cache<String, PlayerEnum>>,
    /// Channel cache (channel_id -> Channel)
    channel_cache: Arc<Cache<String, Channel>>,
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

    pub fn get_player_cache(&self) -> Arc<Cache<String, PlayerEnum>> {
        self.player_cache.clone()
    }

    pub fn get_channel_cache(&self) -> Arc<Cache<String, Channel>> {
        self.channel_cache.clone()
    }

    /// Get a specific channel by ID
    pub async fn get_channel(&self, channel_id: &str) -> Option<Channel> {
        self.channel_cache.get(channel_id).await
    }

    /// List all channels
    pub fn list_channels(&self) -> Vec<Channel> {
        self.channel_cache.iter().map(|(_, channel)| channel).collect()
    }

    /// Create a new channel and insert it into the cache
    pub async fn create_channel(&self, channel: Channel) {
        self.channel_cache.insert(channel.id(), channel).await;
    }

    /// Delete a channel by ID
    pub async fn delete_channel(&self, channel_id: &str) {
        self.channel_cache.remove(channel_id).await;
    }

    /// Rename a channel
    pub async fn rename_channel(&self, channel_id: &str, new_name: String) -> bool {
        if let Some(mut channel) = self.channel_cache.get(channel_id).await {
            channel.rename(new_name);
            self.channel_cache.insert(channel_id.to_string(), channel).await;
            true
        } else {
            false
        }
    }

    /// Add a player to a channel
    pub async fn add_player_to_channel(&self, player: ChannelPlayer, channel_id: &str) {
        if let Some(mut channel) = self.channel_cache.get(channel_id).await {
            let _ = channel.add_player(player);
            self.channel_cache.insert(channel_id.to_string(), channel).await;
            tracing::debug!("Added player to channel {}", channel_id);
        }
    }

    /// Remove a player from a specific channel
    pub async fn remove_player_from_channel(&self, player_name: &str, channel_id: &str) {
        if let Some(mut channel) = self.channel_cache.get(channel_id).await {
            let _ = channel.remove_player(player_name);
            self.channel_cache.insert(channel_id.to_string(), channel).await;
            tracing::debug!("Removed player {} from channel {}", player_name, channel_id);
        }
    }

    /// Get all channels a player is currently in
    pub fn get_player_channels(&self, player_name: &str) -> Vec<String> {
        let mut channels = Vec::new();
        for (channel_id, channel) in self.channel_cache.iter() {
            if channel.players.iter().any(|p| p.name == player_name) {
                channels.push((*channel_id).clone());
            }
        }
        channels
    }

    /// Remove a player from all channels (used when player disconnects)
    /// Returns the list of channel IDs the player was removed from
    pub async fn remove_player_from_all_channels(&self, player_name: &str) -> Vec<String> {
        let mut channels_to_update = Vec::new();
        let mut removed_from_channels = Vec::new();

        for (channel_id, channel) in self.channel_cache.iter() {
            if channel.players.iter().any(|p| p.name == player_name) {
                let mut updated_channel = channel.clone();
                let _ = updated_channel.remove_player(player_name);
                let channel_id_str = channel_id.as_str().to_string();
                removed_from_channels.push(channel_id_str.clone());
                channels_to_update.push((channel_id_str, updated_channel));
            }
        }

        for (channel_id, updated_channel) in channels_to_update {
            self.channel_cache.insert(channel_id.clone(), updated_channel).await;
            tracing::debug!("Updated channel {} after player {} left", channel_id, player_name);
        }

        removed_from_channels
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
                                let player = ChannelPlayer {
                                    name: channel_data.name.clone(),
                                    game: None,
                                    gamerpic: None,
                                };
                                self.add_player_to_channel(player, &channel_data.channel).await;

                                tracing::info!(
                                    "Player {} joined channel {}",
                                    channel_data.name,
                                    channel_data.channel
                                );
                            }
                            ChannelEvents::Leave => {
                                self.remove_player_from_channel(&channel_data.name, &channel_data.channel).await;

                                tracing::info!("Player {} left channel {}", channel_data.name, channel_data.channel);
                            }
                            ChannelEvents::Create => {
                                tracing::info!("Channel {} created by {}", channel_data.channel, channel_data.creator.as_deref().unwrap_or("unknown"));
                            },
                            ChannelEvents::Delete => {
                                self.channel_cache.remove(&channel_data.channel).await;

                                tracing::info!("Channel {} deleted", channel_data.channel);
                            }
                            ChannelEvents::Rename => {}
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

    /// Remove a player from the cache when they disconnect
    /// Returns the list of channel IDs the player was removed from
    pub async fn remove_player(&self, player_name: &str) -> Result<Vec<String>, Error> {
        self.player_cache.remove(player_name).await;

        let removed_channels = self.remove_player_from_all_channels(player_name).await;

        tracing::debug!(
            "Removed player {} from caches on disconnect (was in {} channels)",
            player_name,
            removed_channels.len()
        );
        Ok(removed_channels)
    }
}
