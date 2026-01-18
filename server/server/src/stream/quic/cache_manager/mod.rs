use anyhow::Error;
use common::structs::channel::ChannelEvents;
use common::structs::packet::{
    ChannelEventPacket, PacketType, PlayerDataPacket, QuicNetworkPacket,
};
use common::PlayerEnum;
use moka::future::Cache;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// Manages player position cache and channel membership cache
#[derive(Clone)]
pub struct CacheManager {
    /// Player position cache data
    player_cache: Arc<Cache<String, PlayerEnum>>,
    /// Channel membership cache (channel_id -> Set<player_names>)
    channel_membership: Arc<Cache<String, HashSet<String>>>,
}

impl CacheManager {
    pub fn new() -> Self {
        let player_cache = Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(300)) // 5 minutes
                .max_capacity(256)
                .build(),
        );

        let channel_membership = Arc::new(Cache::builder().max_capacity(100).build());

        Self {
            player_cache,
            channel_membership,
        }
    }

    pub fn get_player_cache(&self) -> Arc<Cache<String, PlayerEnum>> {
        self.player_cache.clone()
    }

    pub fn get_channel_membership(&self) -> Arc<Cache<String, HashSet<String>>> {
        self.channel_membership.clone()
    }

    /// Get all players in a specific channel
    pub async fn get_channel_members(&self, channel_id: &str) -> Option<HashSet<String>> {
        self.channel_membership.get(channel_id).await
    }

    /// Add a player to a channel (creates channel if it doesn't exist)
    pub async fn add_player_to_channel(&self, player_name: &str, channel_id: &str) {
        let mut members = self.channel_membership
            .get(channel_id)
            .await
            .unwrap_or_else(HashSet::new);
        
        members.insert(player_name.to_string());
        self.channel_membership.insert(channel_id.to_string(), members).await;
        
        tracing::debug!("Added player {} to channel {}", player_name, channel_id);
    }

    /// Remove a player from a specific channel (cleans up empty channels)
    pub async fn remove_player_from_channel(&self, player_name: &str, channel_id: &str) {
        if let Some(mut members) = self.channel_membership.get(channel_id).await {
            members.remove(player_name);
            
            if members.is_empty() {
                // Remove empty channel
                self.channel_membership.remove(channel_id).await;
                tracing::debug!("Removed empty channel {}", channel_id);
            } else {
                // Update channel with remaining members
                self.channel_membership.insert(channel_id.to_string(), members).await;
            }
            
            tracing::debug!("Removed player {} from channel {}", player_name, channel_id);
        }
    }

    /// Remove a player from all channels (used when player disconnects)
    pub async fn remove_player_from_all_channels(&self, player_name: &str) {
        let mut channels_to_update = Vec::new();
        
        // Find all channels the player is in
        for (channel_id, members) in self.channel_membership.iter() {
            if members.contains(player_name) {
                let mut updated_members = members.clone();
                updated_members.remove(player_name);
                channels_to_update.push((channel_id.as_str().to_string(), updated_members));
            }
        }
        
        // Update or remove channels
        for (channel_id, updated_members) in channels_to_update {
            if updated_members.is_empty() {
                self.channel_membership.remove(&channel_id).await;
                tracing::debug!("Removed empty channel {} after player {} left", channel_id, player_name);
            } else {
                self.channel_membership.insert(channel_id.clone(), updated_members).await;
                tracing::debug!("Updated channel {} after player {} left", channel_id, player_name);
            }
        }
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
                                // Add player to channel membership
                                self.add_player_to_channel(&channel_data.name, &channel_data.channel).await;
                                
                                tracing::info!(
                                    "Player {} joined channel {}",
                                    channel_data.name,
                                    channel_data.channel
                                );
                            }
                            ChannelEvents::Leave => {
                                // Remove player from channel membership
                                self.remove_player_from_channel(&channel_data.name, &channel_data.channel).await;
                                
                                tracing::info!("Player {} left channel {}", channel_data.name, channel_data.channel);
                            }
                            ChannelEvents::Create => {
                                tracing::info!("Channel {} created by {}", channel_data.channel, channel_data.creator.as_deref().unwrap_or("unknown"));
                            },
                            ChannelEvents::Delete => {
                                // O(1) efficient delete operation!
                                self.channel_membership.remove(&channel_data.channel).await;
                                
                                tracing::info!("Channel {} deleted", channel_data.channel);
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

    /// Remove a player from the cache when they disconnect
    /// This is called when a player disconnects to clean up cache entries
    pub async fn remove_player(&self, player_name: &str) -> Result<(), Error> {
        // Remove from player position cache
        self.player_cache.remove(player_name).await;
        
        // Remove from all channels
        self.remove_player_from_all_channels(player_name).await;
        
        tracing::info!("Removed player {} from caches on disconnect", player_name);
        Ok(())
    }
}
