use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

use super::Channel;

#[derive(Clone)]
pub struct ChannelCollection {
    channels: Arc<Cache<String, Channel>>,
}

impl ChannelCollection {
    pub fn new(max_capacity: u64) -> Self {
        let channels = Arc::new(
            Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(86400))
                .build(),
        );
        Self { channels }
    }

    pub async fn get(&self, channel_id: &str) -> Option<Channel> {
        self.channels.get(channel_id).await
    }

    pub fn list(&self) -> Vec<Channel> {
        self.channels.iter().map(|(_, channel)| channel).collect()
    }

    pub fn get_player_channels(&self, identity: &crate::PlayerIdentity) -> Vec<String> {
        let mut result = Vec::new();
        for (channel_id, channel) in self.channels.iter() {
            if channel.contains(identity) {
                result.push(channel_id.to_string());
            }
        }
        result
    }

    pub async fn insert(&self, channel: Channel) {
        self.channels.insert(channel.id(), channel).await;
    }

    pub async fn remove(&self, channel_id: &str) -> Option<Channel> {
        let channel = self.channels.get(channel_id).await;
        self.channels.remove(channel_id).await;
        channel
    }

    pub async fn rename(&self, channel_id: &str, new_name: String) -> bool {
        if let Some(mut channel) = self.channels.get(channel_id).await {
            channel.rename(new_name);
            self.channels.insert(channel_id.to_string(), channel).await;
            true
        } else {
            false
        }
    }

    pub async fn add_player_to_channel(
        &self,
        identity: &crate::PlayerIdentity,
        channel_id: &str,
    ) -> bool {
        if let Some(mut channel) = self.channels.get(channel_id).await {
            channel.add_player(identity.clone());
            self.channels.insert(channel_id.to_string(), channel).await;
            true
        } else {
            false
        }
    }

    pub async fn remove_player_from_channel(
        &self,
        identity: &crate::PlayerIdentity,
        channel_id: &str,
    ) {
        if let Some(mut channel) = self.channels.get(channel_id).await {
            channel.remove_player(identity);
            self.channels.insert(channel_id.to_string(), channel).await;
        }
    }

    /// Every channel this player left, with each channel's creator.
    ///
    /// The creator travels back because the caller fans a `Leave` per channel and every
    /// channel event names its owner. Reading it here is the last point the channel is
    /// still in hand.
    pub async fn remove_player_from_all_channels(
        &self,
        identity: &crate::PlayerIdentity,
    ) -> Vec<(String, crate::PlayerIdentity)> {
        let mut removed_from = Vec::new();
        let mut updates = Vec::new();

        for (channel_id, channel) in self.channels.iter() {
            if channel.contains(identity) {
                let mut updated = channel.clone();
                updated.remove_player(identity);
                let id = channel_id.to_string();
                removed_from.push((id.clone(), updated.creator.clone()));
                updates.push((id, updated));
            }
        }

        for (channel_id, updated) in updates {
            self.channels.insert(channel_id, updated).await;
        }

        removed_from
    }
}
