use bytes::Bytes;
use common::structs::packet::{
    AudioFramePacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::traits::player_data::PlayerData;
use common::PlayerEnum;
use dashmap::DashMap;
use moka::future::Cache;
use std::sync::Arc;
use tokio::sync::mpsc;

pub(crate) enum RoutedPacket {
    // Pre-serialized datagram bytes (AudioFrame, filtered + serialized on input side)
    Serialized(Bytes),
    // Raw packet for OutputStream to serialize (non-audio broadcast)
    Raw(QuicNetworkPacket),
}

pub(crate) struct ConnectionEntry {
    pub player_name: String,
    pub tx: mpsc::UnboundedSender<RoutedPacket>,
}

pub(crate) struct ConnectionRegistry {
    connections: DashMap<Vec<u8>, ConnectionEntry>,
    // player_name -> channel_id (one channel per player)
    player_channel: DashMap<String, String>,
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
        tx: mpsc::UnboundedSender<RoutedPacket>,
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
        let mut dead_keys: Vec<Vec<u8>> = Vec::new();

        for entry in self.connections.iter() {
            if entry.value().tx.send(RoutedPacket::Raw(packet.clone())).is_err() {
                dead_keys.push(entry.key().clone());
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
    ) {
        let sender_name = match &packet.owner {
            Some(owner) => &owner.name,
            None => return,
        };

        let audio_frame: AudioFramePacket = match &packet.data {
            QuicNetworkPacketData::AudioFrame(af) => af.clone(),
            _ => return,
        };

        let sender_channel: Option<String> =
            self.player_channel.get(sender_name).map(|r| r.clone());

        let original_spatial = audio_frame.spatial;

        // Pre-build two serialized variants
        let bytes_spatial = {
            let mut af = audio_frame.clone();
            af.spatial = Some(true);
            let mut p = packet.clone();
            p.data = QuicNetworkPacketData::AudioFrame(af);
            match p.to_datagram() {
                Ok(bytes) => Bytes::from(bytes),
                Err(e) => {
                    tracing::error!("Failed to serialize spatial audio variant: {}", e);
                    return;
                }
            }
        };

        let bytes_channel = {
            let mut af = audio_frame.clone();
            af.spatial = Some(false);
            let mut p = packet.clone();
            p.data = QuicNetworkPacketData::AudioFrame(af);
            match p.to_datagram() {
                Ok(bytes) => Bytes::from(bytes),
                Err(e) => {
                    tracing::error!("Failed to serialize channel audio variant: {}", e);
                    return;
                }
            }
        };

        let mut dead_keys: Vec<Vec<u8>> = Vec::new();

        // Lazily resolved sender position (only needed for proximity path)
        let mut sender_player: Option<PlayerEnum> = None;
        let mut sender_player_resolved = false;

        for entry in self.connections.iter() {
            let recipient_name = &entry.value().player_name;

            if recipient_name == sender_name {
                continue;
            }

            // Channel check FIRST — no position data required
            let recipient_channel: Option<String> =
                self.player_channel.get(recipient_name).map(|r| r.clone());

            let in_same_channel = match (&sender_channel, &recipient_channel) {
                (Some(sc), Some(rc)) => sc == rc,
                _ => false,
            };

            let bytes_to_send = if in_same_channel {
                match original_spatial {
                    Some(true) => &bytes_spatial,
                    Some(false) | None => &bytes_channel,
                }
            } else {
                // Proximity path: position data required
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

                if let Err(e) = sp.can_communicate_with(&recipient_player, broadcast_range) {
                    tracing::debug!(
                        "Audio packet {} -> {} rejected: {}",
                        sender_name,
                        recipient_name,
                        e
                    );
                    continue;
                }

                // Some(false) is REJECTED outside channels
                match original_spatial {
                    Some(false) => continue,
                    Some(true) | None => &bytes_spatial,
                }
            };

            if entry
                .value()
                .tx
                .send(RoutedPacket::Serialized(bytes_to_send.clone()))
                .is_err()
            {
                dead_keys.push(entry.key().clone());
            }
        }

        for key in dead_keys {
            self.unregister(&key);
        }
    }
}
