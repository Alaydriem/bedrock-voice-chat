use bytes::Bytes;
use common::structs::packet::{QuicNetworkPacket, QuicNetworkPacketData};
use common::traits::player_data::PlayerData;
use common::PlayerEnum;
use dashmap::DashMap;
use moka::future::Cache;
use std::sync::Arc;
use tokio::sync::mpsc;

pub(crate) enum RoutedPacket {
    Serialized(Bytes),
}

pub(crate) struct ConnectionEntry {
    pub player_name: String,
    pub tx: mpsc::Sender<RoutedPacket>,
}

pub(crate) struct ConnectionRegistry {
    connections: DashMap<Vec<u8>, ConnectionEntry>,
    // player_name -> channel_id (one channel per player)
    player_channel: DashMap<String, String>,
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            player_channel: DashMap::new(),
        }
    }

    pub fn register(
        &self,
        client_id: Vec<u8>,
        player_name: String,
        tx: mpsc::Sender<RoutedPacket>,
    ) {
        tracing::info!(
            "Registering connection for player: {} (connections: {})",
            player_name,
            self.connections.len() + 1
        );
        self.connections.insert(
            client_id,
            ConnectionEntry { player_name, tx },
        );
    }

    pub fn unregister(&self, client_id: &[u8]) {
        if let Some((_, entry)) = self.connections.remove(client_id) {
            self.player_channel.remove(&entry.player_name);
            tracing::info!(
                "Unregistered connection for player: {} (connections: {})",
                entry.player_name,
                self.connections.len()
            );
        }
    }

    pub fn broadcast_to_all(&self, packet: QuicNetworkPacket) {
        let bytes = match packet.to_datagram() {
            Ok(bytes) => Bytes::from(bytes),
            Err(e) => {
                tracing::error!("Failed to serialize broadcast: {}", e);
                return;
            }
        };

        let mut dead_keys: Vec<Vec<u8>> = Vec::new();

        for entry in self.connections.iter() {
            match entry.value().tx.try_send(RoutedPacket::Serialized(bytes.clone())) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::debug!(
                        "Dropping broadcast packet for player {} (channel full)",
                        entry.value().player_name,
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    dead_keys.push(entry.key().clone());
                }
            }
        }

        for key in dead_keys {
            self.unregister(&key);
        }
    }

    pub fn update_player_channel(&self, player_name: String, channel_id: String) {
        self.player_channel.insert(player_name, channel_id);
    }

    pub fn remove_player_channel(&self, player_name: &str) {
        self.player_channel.remove(player_name);
    }

    pub fn remove_channel(&self, channel_id: &str) {
        self.player_channel.retain(|_, v| v != channel_id);
    }

    pub async fn route_audio_frame(
        &self,
        packet: &QuicNetworkPacket,
        player_cache: &Arc<Cache<String, PlayerEnum>>,
        broadcast_range: f32,
        deafen_distance: f32,
    ) {
        let sender_name = match &packet.owner {
            Some(owner) => &owner.name,
            None => return,
        };

        let audio_frame = match &packet.data {
            QuicNetworkPacketData::AudioFrame(af) => af,
            _ => return,
        };

        let sender_channel: Option<String> =
            self.player_channel.get(sender_name).map(|r| r.clone());

        let original_spatial = audio_frame.spatial;
        let has_sender = audio_frame.sender.is_some();

        tracing::debug!(
            "route_audio_frame: sender={} original_spatial={:?} has_sender={} sender_channel={:?}",
            sender_name,
            original_spatial,
            has_sender,
            sender_channel,
        );

        // Pre-build serialized variants (single clone, mutate in-place between serializations)
        let mut p = packet.clone();

        let bytes_spatial: Option<Bytes> = {
            if let QuicNetworkPacketData::AudioFrame(ref mut af) = p.data {
                af.spatial = Some(true);
            }
            p.to_datagram().ok().map(Bytes::from)
        };

        let bytes_channel: Option<Bytes> = {
            if let QuicNetworkPacketData::AudioFrame(ref mut af) = p.data {
                af.spatial = Some(false);
            }
            p.to_datagram().ok().map(Bytes::from)
        };

        if bytes_spatial.is_none() && bytes_channel.is_none() {
            return;
        }

        // Snapshot connections to release DashMap shard locks before any .await
        let snapshot: Vec<(Vec<u8>, String, mpsc::Sender<RoutedPacket>)> = self
            .connections
            .iter()
            .map(|entry| {
                (
                    entry.key().clone(),
                    entry.value().player_name.clone(),
                    entry.value().tx.clone(),
                )
            })
            .collect();

        let mut dead_keys: Vec<Vec<u8>> = Vec::new();

        let mut sender_player: Option<PlayerEnum> = None;
        let mut sender_player_resolved = false;

        for (client_id, recipient_name, tx) in &snapshot {
            if recipient_name == sender_name {
                continue;
            }

            let recipient_channel: Option<String> =
                self.player_channel.get(recipient_name).map(|r| r.clone());

            let in_same_channel = match (&sender_channel, &recipient_channel) {
                (Some(sc), Some(rc)) => sc == rc,
                _ => false,
            };

            let bytes_to_send = if in_same_channel {
                tracing::debug!(
                    "route_audio_frame: {} -> {} IN_CHANNEL spatial={:?}",
                    sender_name,
                    recipient_name,
                    original_spatial,
                );
                match original_spatial {
                    // None = unset by client, treat as spatial (default behavior)
                    None | Some(true) => match &bytes_spatial {
                        Some(b) => b,
                        None => continue,
                    },
                    Some(false) => match &bytes_channel {
                        Some(b) => b,
                        None => continue,
                    },
                }
            } else {
                if !sender_player_resolved {
                    sender_player = match &audio_frame.sender {
                        Some(player) => Some(player.clone()),
                        None => player_cache.get(sender_name).await,
                    };
                    sender_player_resolved = true;
                }

                let sp = match &sender_player {
                    Some(p) => p,
                    None => continue,
                };

                let recipient_player = match player_cache.get(recipient_name).await {
                    Some(player) => player,
                    None => continue,
                };

                if sp.get_game() != recipient_player.get_game() {
                    continue;
                }

                let effective_range = if sp.is_deafened() {
                    deafen_distance
                } else {
                    broadcast_range
                };

                if let Err(e) = sp.can_communicate_with(&recipient_player, effective_range) {
                    tracing::debug!(
                        "Audio packet {} -> {} rejected: {}",
                        sender_name,
                        recipient_name,
                        e
                    );
                    continue;
                }

                // Some(false) is rejected outside channels
                match original_spatial {
                    Some(false) => continue,
                    Some(true) | None => match &bytes_spatial {
                        Some(b) => b,
                        None => continue,
                    },
                }
            };

            match tx.try_send(RoutedPacket::Serialized(bytes_to_send.clone())) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::debug!(
                        "Dropping audio packet for player {} (channel full)",
                        recipient_name,
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    dead_keys.push(client_id.clone());
                }
            }
        }

        for key in dead_keys {
            self.unregister(&key);
        }
    }
}
